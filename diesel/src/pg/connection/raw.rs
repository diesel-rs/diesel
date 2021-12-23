#![allow(clippy::too_many_arguments)]

extern crate pq_sys;

use self::pq_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{ptr, str};

use crate::result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    internal_connection: NonNull<PGconn>,
}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        let connection_string = CString::new(database_url)?;
        let connection_ptr = unsafe { PQconnectdb(connection_string.as_ptr()) };
        let connection_status = unsafe { PQstatus(connection_ptr) };

        match connection_status {
            ConnStatusType::CONNECTION_OK => {
                let connection_ptr = unsafe { NonNull::new_unchecked(connection_ptr) };
                Ok(RawConnection {
                    internal_connection: connection_ptr,
                })
            }
            _ => {
                let message = last_error_message(connection_ptr);

                if !connection_ptr.is_null() {
                    // Note that even if the server connection attempt fails (as indicated by PQstatus),
                    // the application should call PQfinish to free the memory used by the PGconn object.
                    // https://www.postgresql.org/docs/current/libpq-connect.html
                    unsafe { PQfinish(connection_ptr) }
                }

                Err(ConnectionError::BadConnection(message))
            }
        }
    }

    pub fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection.as_ptr())
    }

    pub fn set_notice_processor(&self, notice_processor: NoticeProcessor) {
        unsafe {
            PQsetNoticeProcessor(
                self.internal_connection.as_ptr(),
                Some(notice_processor),
                ptr::null_mut(),
            );
        }
    }

    pub unsafe fn exec(&self, query: *const libc::c_char) -> QueryResult<RawResult> {
        RawResult::new(PQexec(self.internal_connection.as_ptr(), query), self)
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
            self.internal_connection.as_ptr(),
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
            self.internal_connection.as_ptr(),
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
        unsafe { PQfinish(self.internal_connection.as_ptr()) };
    }
}

fn last_error_message(conn: *const PGconn) -> String {
    unsafe {
        let error_ptr = PQerrorMessage(conn);
        let bytes = CStr::from_ptr(error_ptr).to_bytes();
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// Internal wrapper around a `*mut PGresult` which is known to be not-null, and
/// have no aliases.  This wrapper is to ensure that it's always properly
/// dropped.
///
/// If `Unique` is ever stabilized, we should use it here.
#[allow(missing_debug_implementations)]
pub struct RawResult(NonNull<PGresult>);

unsafe impl Send for RawResult {}
unsafe impl Sync for RawResult {}

impl RawResult {
    #[allow(clippy::new_ret_no_self)]
    fn new(ptr: *mut PGresult, conn: &RawConnection) -> QueryResult<Self> {
        NonNull::new(ptr).map(RawResult).ok_or_else(|| {
            Error::DatabaseError(
                DatabaseErrorKind::UnableToSendCommand,
                Box::new(conn.last_error_message()),
            )
        })
    }

    pub fn as_ptr(&self) -> *mut PGresult {
        self.0.as_ptr()
    }

    pub fn error_message(&self) -> &str {
        let ptr = unsafe { PQresultErrorMessage(self.0.as_ptr()) };
        let cstr = unsafe { CStr::from_ptr(ptr) };
        cstr.to_str().unwrap_or_default()
    }
}

impl Drop for RawResult {
    fn drop(&mut self) {
        unsafe { PQclear(self.0.as_ptr()) }
    }
}
