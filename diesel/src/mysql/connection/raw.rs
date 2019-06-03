extern crate mysqlclient_sys as ffi;

use std::ffi::CStr;
use std::os::raw as libc;
use std::ptr::{self, NonNull};
use std::sync::{Once, ONCE_INIT};

use super::stmt::Statement;
use super::url::ConnectionOptions;
use crate::result::{ConnectionError, ConnectionResult, QueryResult};

pub struct RawConnection(NonNull<ffi::MYSQL>);

impl RawConnection {
    pub fn new() -> Self {
        perform_thread_unsafe_library_initialization();
        let raw_connection = unsafe { ffi::mysql_init(ptr::null_mut()) };
        // We're trusting https://dev.mysql.com/doc/refman/5.7/en/mysql-init.html
        // that null return always means OOM
        let raw_connection =
            NonNull::new(raw_connection).expect("Insufficient memory to allocate connection");
        let result = RawConnection(raw_connection);

        // This is only non-zero for unrecognized options, which should never happen.
        let charset_result = unsafe {
            ffi::mysql_options(
                result.0.as_ptr(),
                ffi::mysql_option::MYSQL_SET_CHARSET_NAME,
                b"utf8mb4\0".as_ptr() as *const libc::c_void,
            )
        };
        assert_eq!(
            0, charset_result,
            "MYSQL_SET_CHARSET_NAME was not \
             recognized as an option by MySQL. This should never \
             happen."
        );

        result
    }

    pub fn connect(&self, connection_options: &ConnectionOptions) -> ConnectionResult<()> {
        let host = connection_options.host();
        let user = connection_options.user();
        let password = connection_options.password();
        let database = connection_options.database();
        let port = connection_options.port();

        unsafe {
            // Make sure you don't use the fake one!
            ffi::mysql_real_connect(
                self.0.as_ptr(),
                host.map(CStr::as_ptr).unwrap_or_else(|| ptr::null_mut()),
                user.as_ptr(),
                password
                    .map(CStr::as_ptr)
                    .unwrap_or_else(|| ptr::null_mut()),
                database
                    .map(CStr::as_ptr)
                    .unwrap_or_else(|| ptr::null_mut()),
                u32::from(port.unwrap_or(0)),
                ptr::null_mut(),
                0,
            )
        };

        let last_error_message = self.last_error_message();
        if last_error_message.is_empty() {
            Ok(())
        } else {
            Err(ConnectionError::BadConnection(last_error_message))
        }
    }

    pub fn last_error_message(&self) -> String {
        unsafe { CStr::from_ptr(ffi::mysql_error(self.0.as_ptr())) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn execute(&self, query: &str) -> QueryResult<()> {
        unsafe {
            // Make sure you don't use the fake one!
            ffi::mysql_real_query(
                self.0.as_ptr(),
                query.as_ptr() as *const libc::c_char,
                query.len() as libc::c_ulong,
            );
        }
        self.did_an_error_occur()?;
        self.flush_pending_results()?;
        Ok(())
    }

    pub fn enable_multi_statements<T, F>(&self, f: F) -> QueryResult<T>
    where
        F: FnOnce() -> QueryResult<T>,
    {
        unsafe {
            ffi::mysql_set_server_option(
                self.0.as_ptr(),
                ffi::enum_mysql_set_option::MYSQL_OPTION_MULTI_STATEMENTS_ON,
            );
        }
        self.did_an_error_occur()?;

        let result = f();

        unsafe {
            ffi::mysql_set_server_option(
                self.0.as_ptr(),
                ffi::enum_mysql_set_option::MYSQL_OPTION_MULTI_STATEMENTS_OFF,
            );
        }
        self.did_an_error_occur()?;

        result
    }

    pub fn affected_rows(&self) -> usize {
        let affected_rows = unsafe { ffi::mysql_affected_rows(self.0.as_ptr()) };
        affected_rows as usize
    }

    pub fn prepare(&self, query: &str) -> QueryResult<Statement> {
        let stmt = unsafe { ffi::mysql_stmt_init(self.0.as_ptr()) };
        // It is documented that the only reason `mysql_stmt_init` will fail
        // is because of OOM.
        // https://dev.mysql.com/doc/refman/5.7/en/mysql-stmt-init.html
        let stmt = NonNull::new(stmt).expect("Out of memory creating prepared statement");
        let stmt = Statement::new(stmt);
        stmt.prepare(query)?;
        Ok(stmt)
    }

    fn did_an_error_occur(&self) -> QueryResult<()> {
        use crate::result::DatabaseErrorKind;
        use crate::result::Error::DatabaseError;

        let error_message = self.last_error_message();
        if error_message.is_empty() {
            Ok(())
        } else {
            Err(DatabaseError(
                DatabaseErrorKind::__Unknown,
                Box::new(error_message),
            ))
        }
    }

    fn flush_pending_results(&self) -> QueryResult<()> {
        // We may have a result to process before advancing
        self.consume_current_result()?;
        while self.more_results() {
            self.next_result()?;
            self.consume_current_result()?;
        }
        Ok(())
    }

    fn consume_current_result(&self) -> QueryResult<()> {
        unsafe {
            let res = ffi::mysql_store_result(self.0.as_ptr());
            if !res.is_null() {
                ffi::mysql_free_result(res);
            }
        }
        self.did_an_error_occur()
    }

    fn more_results(&self) -> bool {
        unsafe { ffi::mysql_more_results(self.0.as_ptr()) != 0 }
    }

    fn next_result(&self) -> QueryResult<()> {
        unsafe { ffi::mysql_next_result(self.0.as_ptr()) };
        self.did_an_error_occur()
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe {
            ffi::mysql_close(self.0.as_ptr());
        }
    }
}

/// > In a non-multi-threaded environment, `mysql_init()` invokes
/// > `mysql_library_init()` automatically as necessary. However,
/// > `mysql_library_init()` is not thread-safe in a multi-threaded environment,
/// > and thus neither is `mysql_init()`. Before calling `mysql_init()`, either
/// > call `mysql_library_init()` prior to spawning any threads, or use a mutex
/// > to protect the `mysql_library_init()` call. This should be done prior to
/// > any other client library call.
///
/// <https://dev.mysql.com/doc/refman/5.7/en/mysql-init.html>
static MYSQL_THREAD_UNSAFE_INIT: Once = ONCE_INIT;

fn perform_thread_unsafe_library_initialization() {
    MYSQL_THREAD_UNSAFE_INIT.call_once(|| {
        // mysql_library_init is defined by `#define mysql_library_init mysql_server_init`
        // which isn't picked up by bindgen
        let error_code = unsafe { ffi::mysql_server_init(0, ptr::null_mut(), ptr::null_mut()) };
        if error_code != 0 {
            // FIXME: This is documented as Nonzero if an error occurred.
            // Presumably the value has some sort of meaning that we should
            // reflect in this message. We are going to panic instead of return
            // an error here, since the documentation does not indicate whether
            // it is safe to call this function twice if the first call failed,
            // so I will assume it is not.
            panic!("Unable to perform MySQL global initialization");
        }
    })
}
