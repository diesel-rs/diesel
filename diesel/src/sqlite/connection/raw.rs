extern crate libsqlite3_sys as ffi;

use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{mem, ptr, slice, str};

use super::serialized_value::SerializedValue;
use super::functions::FunctionRow;
use super::Sqlite;
use crate::deserialize::{FromSqlRow, Queryable};
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::HasSqlType;

// Sticking this here for now until I find a better place
pub trait Aggregator<Args>: Default {
    type Output;

    fn step(&mut self, args: Args);
    fn finalize(&self) -> Self::Output;
}

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    pub(crate) internal_connection: NonNull<ffi::sqlite3>,
}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        let mut conn_pointer = ptr::null_mut();
        let database_url = CString::new(database_url.trim_start_matches("sqlite://"))?;
        let connection_status =
            unsafe { ffi::sqlite3_open(database_url.as_ptr(), &mut conn_pointer) };

        match connection_status {
            ffi::SQLITE_OK => {
                let conn_pointer = unsafe { NonNull::new_unchecked(conn_pointer) };
                Ok(RawConnection {
                    internal_connection: conn_pointer,
                })
            }
            err_code => {
                let message = super::error_message(err_code);
                Err(ConnectionError::BadConnection(message.into()))
            }
        }
    }

    pub fn exec(&self, query: &str) -> QueryResult<()> {
        let mut err_msg = ptr::null_mut();
        let query = CString::new(query)?;
        let callback_fn = None;
        let callback_arg = ptr::null_mut();
        unsafe {
            ffi::sqlite3_exec(
                self.internal_connection.as_ptr(),
                query.as_ptr(),
                callback_fn,
                callback_arg,
                &mut err_msg,
            );
        }

        if err_msg.is_null() {
            Ok(())
        } else {
            let msg = convert_to_string_and_free(err_msg);
            let error_kind = DatabaseErrorKind::__Unknown;
            Err(DatabaseError(error_kind, Box::new(msg)))
        }
    }

    pub fn rows_affected_by_last_query(&self) -> usize {
        unsafe { ffi::sqlite3_changes(self.internal_connection.as_ptr()) as usize }
    }

    pub fn register_sql_function<F>(
        &self,
        fn_name: &str,
        num_args: usize,
        deterministic: bool,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(&Self, &[*mut ffi::sqlite3_value]) -> QueryResult<SerializedValue>
            + Send
            + 'static,
    {
        let fn_name = CString::new(fn_name)?;
        let mut flags = ffi::SQLITE_UTF8;
        if deterministic {
            flags |= ffi::SQLITE_DETERMINISTIC;
        }
        let callback_fn = Box::into_raw(Box::new(f));

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                fn_name.as_ptr(),
                num_args as _,
                flags,
                callback_fn as *mut _,
                Some(run_custom_function::<F>),
                None,
                None,
                Some(destroy_boxed_fn::<F>),
            )
        };

        if result == ffi::SQLITE_OK {
            Ok(())
        } else {
            let error_message = super::error_message(result);
            Err(DatabaseError(
                DatabaseErrorKind::__Unknown,
                Box::new(error_message.to_string()),
            ))
        }
    }

    // TODO @thekuom: abstract out some common code with register_sql_function
    pub fn register_aggregate_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
        &self,
        fn_name: &str,
        num_args: usize,
    ) -> QueryResult<()>
    where
        A: Aggregator<Args, Output=Ret> + 'static + Send,
        Args: Queryable<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        let fn_name = CString::new(fn_name)?;
        // Aggregate functions are always deterministic
        let flags = ffi::SQLITE_UTF8 | ffi::SQLITE_DETERMINISTIC;

        // TODO @thekuom: figure out a way to just pass nothing
        let user_data = Box::into_raw(Box::new(0));

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                fn_name.as_ptr(),
                num_args as _,
                flags,
                user_data as *mut _,
                None,
                Some(run_aggregator_step_function::<_, _, _, _, A>),
                Some(run_aggregator_final_function::<_, _, _, _, A>),
                Some(destroy_boxed_type::<i32>),
            )
        };

        if result == ffi::SQLITE_OK {
            Ok(())
        } else {
            let error_message = super::error_message(result);
            Err(DatabaseError(
                    DatabaseErrorKind::__Unknown,
                    Box::new(error_message.to_string()),
            ))
        }
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        use std::thread::panicking;

        let close_result = unsafe { ffi::sqlite3_close(self.internal_connection.as_ptr()) };
        if close_result != ffi::SQLITE_OK {
            let error_message = super::error_message(close_result);
            if panicking() {
                write!(
                    stderr(),
                    "Error closing SQLite connection: {}",
                    error_message
                )
                .expect("Error writing to `stderr`");
            } else {
                panic!("Error closing SQLite connection: {}", error_message);
            }
        }
    }
}

fn convert_to_string_and_free(err_msg: *const libc::c_char) -> String {
    let msg = unsafe {
        let bytes = CStr::from_ptr(err_msg).to_bytes();
        // sqlite is documented to return utf8 strings here
        str::from_utf8_unchecked(bytes).into()
    };
    unsafe { ffi::sqlite3_free(err_msg as *mut libc::c_void) };
    msg
}

#[allow(warnings)]
extern "C" fn run_custom_function<F>(
    ctx: *mut ffi::sqlite3_context,
    num_args: libc::c_int,
    value_ptr: *mut *mut ffi::sqlite3_value,
) where
    F: FnMut(&RawConnection, &[*mut ffi::sqlite3_value]) -> QueryResult<SerializedValue>
        + Send
        + 'static,
{
    static NULL_DATA_ERR: &str = "An unknown error occurred. sqlite3_user_data returned a null pointer. This should never happen.";
    static NULL_CONN_ERR: &str = "An unknown error occurred. sqlite3_context_db_handle returned a null pointer. This should never happen.";

    unsafe {
        let data_ptr = ffi::sqlite3_user_data(ctx);
        let data_ptr = data_ptr as *mut F;
        let f = match data_ptr.as_mut() {
            Some(f) => f,
            None => {
                ffi::sqlite3_result_error(
                    ctx,
                    NULL_DATA_ERR.as_ptr() as *const _ as *const _,
                    NULL_DATA_ERR.len() as _,
                );
                return;
            }
        };

        let args = slice::from_raw_parts(value_ptr, num_args as _);
        let conn = match NonNull::new(ffi::sqlite3_context_db_handle(ctx)) {
            Some(conn) => RawConnection {
                internal_connection: conn,
            },
            None => {
                ffi::sqlite3_result_error(
                    ctx,
                    NULL_DATA_ERR.as_ptr() as *const _ as *const _,
                    NULL_DATA_ERR.len() as _,
                );
                return;
            }
        };
        match f(&conn, args) {
            Ok(value) => value.result_of(ctx),
            Err(e) => {
                let msg = e.to_string();
                ffi::sqlite3_result_error(ctx, msg.as_ptr() as *const _, msg.len() as _);
            }
        }

        mem::forget(conn);
    }
}

// Need a custom option type here, because the std lib one does not have guarantees about the discriminate values?
#[repr(u8)]
enum OptionalAggregator<A> {
    // Discriminant is 0
    #[allow(dead_code)]
    None,
    Some(A),
}

#[allow(warnings)]
extern "C" fn run_aggregator_step_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
    ctx: *mut ffi::sqlite3_context,
    num_args: libc::c_int,
    value_ptr: *mut *mut ffi::sqlite3_value,
)
where
    A: Aggregator<Args, Output=Ret> + 'static + Send,
    Args: Queryable<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    static NULL_AG_CTX_ERR: &str = "An unknown error occurred. sqlite3_aggregate_context returned a null pointer. This should never happen.";

    unsafe {
        let aggregate_context = ffi::sqlite3_aggregate_context(ctx, std::mem::size_of::<OptionalAggregator<A>>() as i32);
        let mut aggregate_context = NonNull::new(aggregate_context as *mut OptionalAggregator<A>);
        let aggregator = match aggregate_context {
            // Check with someone that does more unsafe code reviews if this is ok. We do
            // only access the tag of an zeroed variant here, right? That tag is guaranteed to be zero for
            // our case and this variant.
            Some(ref mut a) => match a.as_mut() {
                &mut OptionalAggregator::Some(ref mut agg) => agg,
                &mut OptionalAggregator::None => {
                    // Not sure if we should use `write_unaligned` instead
                    ptr::write(a.as_mut(), OptionalAggregator::Some(A::default()));
                    if let &mut OptionalAggregator::Some(ref mut agg) = a.as_mut() {
                        agg
                    } else {
                        unreachable!("We've written the aggregator above to that location, it must be there")
                    }
                }
            },
            None => {
                ffi::sqlite3_result_error(
                    ctx,
                    NULL_AG_CTX_ERR.as_ptr() as *const _ as *const _,
                    NULL_AG_CTX_ERR.len() as _,
                );
                return;
            }
        };

        let mut f = |args: &[*mut ffi::sqlite3_value]| {
            let mut row = FunctionRow { args };
            let args_row = Args::Row::build_from_row(&mut row).map_err(Error::DeserializationError).unwrap(); // TODO @thekuom: error handling
            let args = Args::build(args_row);

            aggregator.step(args);
        };

        let args = slice::from_raw_parts(value_ptr, num_args as _);
        f(args);
    }
}

extern "C" fn run_aggregator_final_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
    ctx: *mut ffi::sqlite3_context,
)
where
    A: Aggregator<Args, Output=Ret> + 'static + Send,
    Args: Queryable<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    static NULL_AG_CTX_ERR: &str = "An unknown error occurred. sqlite3_aggregate_context returned a null pointer. This should never happen.";

    unsafe {
        let aggregate_context = ffi::sqlite3_aggregate_context(ctx, 0);
        let aggregate_context = NonNull::new(aggregate_context as *mut OptionalAggregator<A>);
        // TODO @thekuom: abstract this out
        let aggregator = match aggregate_context {
            // Check with someone that does more unsafe code reviews if this is ok. We do
            // only access the tag of an zeroed variant here, right? That tag is guaranteed to be zero for
            // our case and this variant.
            Some(ref a) => match a.as_ref() {
                &OptionalAggregator::Some(ref agg) => agg,
                &OptionalAggregator::None => {
                    ffi::sqlite3_result_error(
                        ctx,
                        NULL_AG_CTX_ERR.as_ptr() as *const _ as *const _,
                        NULL_AG_CTX_ERR.len() as _,
                        );
                    return;
                }
            },
            None => {
                ffi::sqlite3_result_error(
                    ctx,
                    NULL_AG_CTX_ERR.as_ptr() as *const _ as *const _,
                    NULL_AG_CTX_ERR.len() as _,
                    );
                return;
            }
        };

        let result = aggregator.finalize();

        let f = || -> QueryResult<SerializedValue> {
            let mut buf = Output::new(Vec::new(), &());
            let is_null = result.to_sql(&mut buf).map_err(Error::SerializationError).unwrap(); // TODO @thekuom: error handling

            let bytes = if let IsNull::Yes = is_null {
                None
            } else {
                Some(buf.into_inner())
            };

            Ok(SerializedValue {
                ty: Sqlite::metadata(&()),
                data: bytes,
            })
        };

        match f() {
            Ok(value) => value.result_of(ctx),
            Err(e) => {
                let msg = e.to_string();
                ffi::sqlite3_result_error(ctx, msg.as_ptr() as *const _, msg.len() as _);
            }
        }
    }
}

extern "C" fn destroy_boxed_fn<F>(data: *mut libc::c_void)
where
    F: FnMut(&RawConnection, &[*mut ffi::sqlite3_value]) -> QueryResult<SerializedValue>
        + Send
        + 'static,
{
    destroy_boxed_type::<F>(data);
}

extern "C" fn destroy_boxed_type<T>(data: *mut libc::c_void)
{
    let ptr = data as *mut T;
    unsafe { Box::from_raw(ptr) };
}
