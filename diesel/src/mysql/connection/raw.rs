extern crate mysqlclient_sys as ffi;

use std::ffi::CStr;
use std::os::raw as libc;
use std::ptr;
use std::sync::{Once, ONCE_INIT};

use result::{ConnectionError, ConnectionResult, QueryResult};
use super::url::ConnectionOptions;
use super::stmt::Statement;

pub struct RawConnection(*mut ffi::MYSQL);

impl RawConnection {
    pub fn new() -> Self {
        perform_thread_unsafe_library_initialization();
        let raw_connection = unsafe { ffi::mysql_init(ptr::null_mut()) };
        if raw_connection.is_null() {
            // We're trusting https://dev.mysql.com/doc/refman/5.7/en/mysql-init.html
            // that null return always means OOM
            panic!("Insufficient memory to allocate connection");
        }
        let result = RawConnection(raw_connection);

        // This is only non-zero for unrecognized options, which should never happen.
        let charset_result = unsafe {
            ffi::mysql_options(
                result.0,
                ffi::mysql_option::MYSQL_SET_CHARSET_NAME,
                b"utf8mb4\0".as_ptr() as *const libc::c_void,
            )
        };
        assert_eq!(
            0,
            charset_result,
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
                self.0,
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
        unsafe { CStr::from_ptr(ffi::mysql_error(self.0)) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn execute(&self, query: &str) -> QueryResult<()> {
        unsafe {
            // Make sure you don't use the fake one!
            ffi::mysql_real_query(
                self.0,
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
                self.0,
                ffi::enum_mysql_set_option::MYSQL_OPTION_MULTI_STATEMENTS_ON,
            );
        }
        self.did_an_error_occur()?;

        let result = f();

        unsafe {
            ffi::mysql_set_server_option(
                self.0,
                ffi::enum_mysql_set_option::MYSQL_OPTION_MULTI_STATEMENTS_OFF,
            );
        }
        self.did_an_error_occur()?;

        result
    }

    pub fn affected_rows(&self) -> usize {
        let affected_rows = unsafe { ffi::mysql_affected_rows(self.0) };
        affected_rows as usize
    }

    pub fn prepare(&self, query: &str) -> QueryResult<Statement> {
        let stmt = unsafe { ffi::mysql_stmt_init(self.0) };
        // It is documented that the only reason `mysql_stmt_init` will fail
        // is because of OOM.
        // https://dev.mysql.com/doc/refman/5.7/en/mysql-stmt-init.html
        assert!(!stmt.is_null(), "Out of memory creating prepared statement");
        let stmt = Statement::new(stmt);
        try!(stmt.prepare(query));
        Ok(stmt)
    }

    fn did_an_error_occur(&self) -> QueryResult<()> {
        use result::DatabaseErrorKind;
        use result::Error::DatabaseError;

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
        while self.next_result()? {
            self.consume_current_result()?;
        }
        // next_result returns whether we've advanced to the *last* one, not
        // whether we're completely done.
        self.consume_current_result()?;
        Ok(())
    }

    fn consume_current_result(&self) -> QueryResult<()> {
        unsafe {
            let res = ffi::mysql_store_result(self.0);
            if !res.is_null() {
                ffi::mysql_free_result(res);
            }
        }
        self.did_an_error_occur()
    }

    /// Calls `mysql_next_result` and returns whether there are more results
    /// after this one.
    fn next_result(&self) -> QueryResult<bool> {
        let more_results = unsafe { ffi::mysql_next_result(self.0) == 0 };
        self.did_an_error_occur()?;
        Ok(more_results)
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe {
            ffi::mysql_close(self.0);
        }
    }
}

/// > In a nonmulti-threaded environment, `mysql_init()` invokes
/// > `mysql_library_init()` automatically as necessary. However,
/// > `mysql_library_init()` is not thread-safe in a multi-threaded environment,
/// > and thus neither is `mysql_init()`. Before calling `mysql_init()`, either
/// > call `mysql_library_init()` prior to spawning any threads, or use a mutex
/// > ot protect the `mysql_library_init()` call. This should be done prior to
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
