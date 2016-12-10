extern crate pq_sys;
extern crate libc;

use self::pq_sys::*;
use std::ffi::{CString, CStr};
use std::{str, ptr};

use result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    internal_connection: *mut PGconn,
}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        let connection_string = try!(CString::new(database_url));
        let connection_ptr = unsafe { PQconnectdb(connection_string.as_ptr()) };
        let connection_status = unsafe { PQstatus(connection_ptr) };

        match connection_status {
            CONNECTION_OK => {
                Ok(RawConnection {
                    internal_connection: connection_ptr,
                })
            }
            _ => {
                let message = last_error_message(connection_ptr);
                Err(ConnectionError::BadConnection(message))
            }
        }
    }

    pub fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection)
    }

    pub fn escape_identifier(&self, identifier: &str) -> Result<PgString, String> {
        let result_ptr = unsafe { PQescapeIdentifier(
            self.internal_connection,
            identifier.as_ptr() as *const libc::c_char,
            identifier.len() as libc::size_t,
        ) };

        if result_ptr.is_null() {
            Err(last_error_message(self.internal_connection))
        } else {
            unsafe {
                Ok(PgString::new(result_ptr))
            }
        }
    }

    pub fn set_notice_processor(&self, notice_processor: NoticeProcessor) {
        unsafe {
            PQsetNoticeProcessor(self.internal_connection, Some(notice_processor), ptr::null_mut());
        }
    }

    pub unsafe fn exec(&self, query: *const libc::c_char) -> QueryResult<RawResult> {
        RawResult::new(PQexec(self.internal_connection, query), self)
    }

    pub unsafe fn exec_params(
        &self,
        query: *const libc::c_char,
        param_count: libc::c_int,
        param_types: *const Oid,
        param_values: *const *const libc::c_char,
        param_lengths: *const libc::c_int,
        param_formats: *const libc::c_int,
        result_format: libc::c_int,
    ) -> QueryResult<RawResult> {
        let ptr = PQexecParams(
            self.internal_connection,
            query,
            param_count,
            param_types,
            param_values,
            param_lengths,
            param_formats,
            result_format,
        );
        RawResult::new(ptr, self)
    }

    pub unsafe fn exec_prepared(
        &self,
        stmt_name: *const libc::c_char,
        param_count: libc::c_int,
        param_values: *const *const libc::c_char,
        param_lengths: *const libc::c_int,
        param_formats: *const libc::c_int,
        result_format: libc::c_int,
    ) -> QueryResult<RawResult> {
        let ptr = PQexecPrepared(
            self.internal_connection,
            stmt_name,
            param_count,
            param_values,
            param_lengths,
            param_formats,
            result_format,
        );
        RawResult::new(ptr, self)
    }

    pub unsafe fn prepare(
        &self,
        stmt_name: *const libc::c_char,
        query: *const libc::c_char,
        param_count: libc::c_int,
        param_types: *const Oid,
    ) -> QueryResult<RawResult> {
        let ptr = PQprepare(
            self.internal_connection,
            stmt_name,
            query,
            param_count,
            param_types,
        );
        RawResult::new(ptr, self)
    }
}

pub type NoticeProcessor = extern "C" fn(arg: *mut libc::c_void, message: *const libc::c_char);

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe { PQfinish(self.internal_connection) };
    }
}

fn last_error_message(conn: *const PGconn) -> String {
    unsafe {
        let error_ptr = PQerrorMessage(conn);
        let bytes = CStr::from_ptr(error_ptr).to_bytes();
        str::from_utf8_unchecked(bytes).to_string()
    }
}

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct PgString {
    pg_str: *mut libc::c_char,
}

impl PgString {
    unsafe fn new(ptr: *mut libc::c_char) -> Self {
        PgString {
            pg_str: ptr,
        }
    }
}

impl ::std::ops::Deref for PgString {
    type Target = str;

    fn deref(&self) -> &str {
        unsafe {
            let c_string = CStr::from_ptr(self.pg_str);
            str::from_utf8_unchecked(c_string.to_bytes())
        }
    }
}

impl Drop for PgString {
    fn drop(&mut self) {
        unsafe {
            PQfreemem(self.pg_str as *mut libc::c_void)
        }
    }
}

/// Internal wrapper around a `*mut PGresult` which is known to be not-null, and
/// have no aliases.  This wrapper is to ensure that it's always properly
/// dropped.
///
/// If `Unique` is ever stabilized, we should use it here.
#[allow(missing_debug_implementations)]
pub struct RawResult(*mut PGresult);

unsafe impl Send for RawResult {}
unsafe impl Sync for RawResult {}

impl RawResult {
    fn new(ptr: *mut PGresult, conn: &RawConnection) -> QueryResult<Self> {
        if ptr.is_null() {
            Err(Error::DatabaseError(
                DatabaseErrorKind::UnableToSendCommand,
                Box::new(conn.last_error_message()),
            ))
        } else {
            Ok(RawResult(ptr))
        }
    }

    pub fn as_ptr(&self) -> *mut PGresult {
        self.0
    }
}

impl Drop for RawResult {
    fn drop(&mut self) {
        unsafe { PQclear(self.0) }
    }
}
