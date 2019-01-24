extern crate libsqlite3_sys as ffi;

use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{mem, ptr, slice, str};

use super::serialized_value::SerializedValue;
use result::Error::DatabaseError;
use result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    pub(crate) internal_connection: NonNull<ffi::sqlite3>,
}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        let mut conn_pointer = ptr::null_mut();
        let database_url = CString::new(database_url)?;
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

extern "C" fn destroy_boxed_fn<F>(data: *mut libc::c_void)
where
    F: FnMut(&RawConnection, &[*mut ffi::sqlite3_value]) -> QueryResult<SerializedValue>
        + Send
        + 'static,
{
    let ptr = data as *mut F;
    unsafe { Box::from_raw(ptr) };
}
