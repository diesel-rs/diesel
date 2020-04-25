extern crate libsqlite3_sys as ffi;

use std::ffi::{CStr, CString, NulError};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{mem, ptr, slice, str};

use super::functions::{build_sql_function_args, process_sql_function_result};
use super::serialized_value::SerializedValue;
use super::{Sqlite, SqliteAggregateFunction};
use crate::deserialize::Queryable;
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, IntoNullable};

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
        let fn_name = Self::get_fn_name(fn_name)?;
        let flags = Self::get_flags(deterministic);
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

        Self::process_sql_function_result(result)
    }

    pub fn register_aggregate_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
        &self,
        fn_name: &str,
        num_args: usize,
    ) -> QueryResult<()>
    where
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send,
        Args: Queryable<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        RetSqlType: IntoNullable<Nullable = RetSqlType>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        let fn_name = Self::get_fn_name(fn_name)?;
        let flags = Self::get_flags(false);

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                fn_name.as_ptr(),
                num_args as _,
                flags,
                ptr::null_mut(),
                None,
                Some(run_aggregator_step_function::<_, _, _, _, A>),
                Some(run_aggregator_final_function::<_, _, _, _, A>),
                None,
            )
        };

        Self::process_sql_function_result(result)
    }

    fn get_fn_name(fn_name: &str) -> Result<CString, NulError> {
        Ok(CString::new(fn_name)?)
    }

    fn get_flags(deterministic: bool) -> i32 {
        let mut flags = ffi::SQLITE_UTF8;
        if deterministic {
            flags |= ffi::SQLITE_DETERMINISTIC;
        }
        flags
    }

    fn process_sql_function_result(result: i32) -> Result<(), Error> {
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

// Need a custom option type here, because the std lib one does not have guarantees about the discriminate values
// See: https://github.com/rust-lang/rfcs/blob/master/text/2195-really-tagged-unions.md#opaque-tags
#[repr(u8)]
enum OptionalAggregator<A> {
    // Discriminant is 0
    None,
    Some(A),
}

#[allow(warnings)]
extern "C" fn run_aggregator_step_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
    ctx: *mut ffi::sqlite3_context,
    num_args: libc::c_int,
    value_ptr: *mut *mut ffi::sqlite3_value,
) where
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send,
    Args: Queryable<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    unsafe {
        let aggregate_context = ffi::sqlite3_aggregate_context(
            ctx,
            std::mem::size_of::<OptionalAggregator<A>>() as i32,
        );
        let mut aggregate_context = NonNull::new(aggregate_context as *mut OptionalAggregator<A>);
        let aggregator = match aggregate_context.map(|a| &mut *a.as_ptr()) {
            Some(&mut OptionalAggregator::Some(ref mut agg)) => agg,
            Some(mut a_ptr @ &mut OptionalAggregator::None) => {
                ptr::write_unaligned(a_ptr as *mut _, OptionalAggregator::Some(A::default()));
                if let &mut OptionalAggregator::Some(ref mut agg) = a_ptr {
                    agg
                } else {
                    unreachable!(
                        "We've written the aggregator above to that location, it must be there"
                    )
                }
            }
            None => {
                null_aggregate_context_error(ctx);
                return;
            }
        };

        let mut f = |args: &[*mut ffi::sqlite3_value]| -> Result<(), Error> {
            let args = build_sql_function_args::<ArgsSqlType, Args>(args)?;

            Ok(aggregator.step(args))
        };

        let args = slice::from_raw_parts(value_ptr, num_args as _);
        match f(args) {
            Err(e) => {
                let msg = e.to_string();
                ffi::sqlite3_result_error(ctx, msg.as_ptr() as *const _, msg.len() as _);
            }
            _ => (),
        };
    }
}

extern "C" fn run_aggregator_final_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
    ctx: *mut ffi::sqlite3_context,
) where
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send,
    Args: Queryable<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    RetSqlType: IntoNullable<Nullable = RetSqlType>,
    Sqlite: HasSqlType<RetSqlType>,
{
    unsafe {
        // Within the xFinal callback, it is customary to set nBytes to 0 so no pointless memory
        // allocations occur
        // See: https://www.sqlite.org/c3ref/aggregate_context.html
        let aggregate_context = ffi::sqlite3_aggregate_context(ctx, 0);
        let mut aggregate_context = NonNull::new(aggregate_context as *mut OptionalAggregator<A>);
        let aggregator = match aggregate_context {
            Some(ref mut a) => match std::mem::replace(a.as_mut(), OptionalAggregator::None) {
                OptionalAggregator::Some(agg) => Some(agg),
                OptionalAggregator::None => unreachable!("We've written to the aggregator in the xStep callback. If xStep was never called, then ffi::sqlite_aggregate_context() would have returned a NULL pointer")
            },
            None => None,
        };

        let result = A::finalize(aggregator);

        match process_sql_function_result::<RetSqlType, Option<Ret>>(result) {
            Ok(value) => value.result_of(ctx),
            Err(e) => {
                let msg = e.to_string();
                ffi::sqlite3_result_error(ctx, msg.as_ptr() as *const _, msg.len() as _);
            }
        }
    }
}

unsafe fn null_aggregate_context_error(ctx: *mut ffi::sqlite3_context) {
    static NULL_AG_CTX_ERR: &str = "An unknown error occurred. sqlite3_aggregate_context returned a null pointer. This should never happen.";

    ffi::sqlite3_result_error(
        ctx,
        NULL_AG_CTX_ERR.as_ptr() as *const _ as *const _,
        NULL_AG_CTX_ERR.len() as _,
    );
}

extern "C" fn destroy_boxed_fn<F>(data: *mut libc::c_void)
where
    F: FnMut(&RawConnection, &[*mut ffi::sqlite3_value]) -> QueryResult<SerializedValue>
        + Send
        + 'static,
{
    let ptr = data as *mut F;
    unsafe { Box::from_raw(ptr) };
}
