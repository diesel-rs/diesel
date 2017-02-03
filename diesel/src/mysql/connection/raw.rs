extern crate mysqlclient_sys as ffi;

use std::ffi::CStr;
use std::os::{raw as libc};
use std::ptr;
use std::sync::{Once, ONCE_INIT};

use result::{ConnectionResult, ConnectionError};
use super::url::ConnectionOptions;

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
        let charset_result = unsafe { ffi::mysql_options(
            result.0,
            ffi::mysql_option::MYSQL_SET_CHARSET_NAME,
            b"utf8mb4\0".as_ptr() as *const libc::c_void,
        ) };
        assert_eq!(0, charset_result, "MYSQL_SET_CHARSET_NAME was not \
                   recognized as an option by MySQL. This should never \
                   happen.");

        result
    }

    pub fn connect(&self, connection_options: ConnectionOptions) -> ConnectionResult<()> {
        let host = try!(connection_options.host());
        let user = try!(connection_options.user());
        let password = try!(connection_options.password());
        let database = try!(connection_options.database());
        let port = connection_options.port();

        unsafe {
            // Make sure you don't use the fake one!
            ffi::mysql_real_connect(
                self.0,
                host.map(|x| x.as_ptr()).unwrap_or(ptr::null_mut()),
                user.as_ptr(),
                password.map(|x| x.as_ptr()).unwrap_or(ptr::null_mut()),
                database.map(|x| x.as_ptr()).unwrap_or(ptr::null_mut()),
                port.unwrap_or(0) as u32,
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
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe { ffi::mysql_close(self.0); }
    }
}

/// In a nonmulti-threaded environment, mysql_init() invokes mysql_library_init() automatically as
/// necessary. However, mysql_library_init() is not thread-safe in a multi-threaded environment,
/// and thus neither is mysql_init(). Before calling mysql_init(), either call mysql_library_init()
/// prior to spawning any threads, or use a mutex to protect the mysql_library_init() call. This
/// should be done prior to any other client library call.
///
/// https://dev.mysql.com/doc/refman/5.7/en/mysql-init.html
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
