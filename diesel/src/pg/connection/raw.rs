#![allow(clippy::too_many_arguments)]
#![allow(unsafe_code)] // ffi code

extern crate pq_sys;

use self::pq_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{ptr, str};

use crate::result::*;

use super::result::PgResult;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub(super) struct RawConnection {
    pub(super) internal_connection: NonNull<PGconn>,
}

impl RawConnection {
    pub(super) fn establish(database_url: &str) -> ConnectionResult<Self> {
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

    pub(super) fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection.as_ptr())
    }

    pub(super) fn set_notice_processor(&self, notice_processor: NoticeProcessor) {
        unsafe {
            PQsetNoticeProcessor(
                self.internal_connection.as_ptr(),
                Some(notice_processor),
                ptr::null_mut(),
            );
        }
    }

    pub(super) unsafe fn exec(&self, query: *const libc::c_char) -> QueryResult<RawResult> {
        RawResult::new(PQexec(self.internal_connection.as_ptr(), query), self)
    }

    pub(super) unsafe fn send_query_prepared(
        &self,
        stmt_name: *const libc::c_char,
        param_count: libc::c_int,
        param_values: *const *const libc::c_char,
        param_lengths: *const libc::c_int,
        param_formats: *const libc::c_int,
        result_format: libc::c_int,
    ) -> QueryResult<()> {
        let res = PQsendQueryPrepared(
            self.internal_connection.as_ptr(),
            stmt_name,
            param_count,
            param_values,
            param_lengths,
            param_formats,
            result_format,
        );
        if res == 1 {
            Ok(())
        } else {
            Err(Error::DatabaseError(
                DatabaseErrorKind::UnableToSendCommand,
                Box::new(self.last_error_message()),
            ))
        }
    }

    pub(super) unsafe fn prepare(
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

    /// This is reasonably inexpensive as it just accesses variables internal to the connection
    /// that are kept up to date by the `ReadyForQuery` messages from the PG server
    pub(super) fn transaction_status(&self) -> PgTransactionStatus {
        unsafe { PQtransactionStatus(self.internal_connection.as_ptr()) }.into()
    }

    pub(super) fn get_status(&self) -> ConnStatusType {
        unsafe { PQstatus(self.internal_connection.as_ptr()) }
    }

    pub(crate) fn get_next_result(&self) -> Result<Option<PgResult>, Error> {
        let res = unsafe { PQgetResult(self.internal_connection.as_ptr()) };
        if res.is_null() {
            Ok(None)
        } else {
            let raw = RawResult::new(res, self)?;
            Ok(Some(PgResult::new(raw, self)?))
        }
    }

    pub(crate) fn enable_row_by_row_mode(&self) -> QueryResult<()> {
        let res = unsafe { PQsetSingleRowMode(self.internal_connection.as_ptr()) };
        if res == 1 {
            Ok(())
        } else {
            Err(Error::DatabaseError(
                DatabaseErrorKind::Unknown,
                Box::new(self.last_error_message()),
            ))
        }
    }

    pub(super) fn put_copy_data(&mut self, buf: &[u8]) -> QueryResult<()> {
        for c in buf.chunks(i32::MAX as usize) {
            let res = unsafe {
                pq_sys::PQputCopyData(
                    self.internal_connection.as_ptr(),
                    c.as_ptr() as *const libc::c_char,
                    c.len() as libc::c_int,
                )
            };
            if res != 1 {
                return Err(Error::DatabaseError(
                    DatabaseErrorKind::Unknown,
                    Box::new(self.last_error_message()),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn finish_copy_from(&self, err: Option<String>) -> QueryResult<()> {
        let error = err.map(CString::new).map(|r| {
            r.unwrap_or_else(|_| {
                CString::new("Error message contains a \\0 byte")
                    .expect("Does not contain a null byte")
            })
        });
        let error = error
            .as_ref()
            .map(|l| l.as_ptr())
            .unwrap_or(std::ptr::null());
        let ret = unsafe { pq_sys::PQputCopyEnd(self.internal_connection.as_ptr(), error) };
        if ret == 1 {
            Ok(())
        } else {
            Err(Error::DatabaseError(
                DatabaseErrorKind::Unknown,
                Box::new(self.last_error_message()),
            ))
        }
    }
}

/// Represents the current in-transaction status of the connection
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum PgTransactionStatus {
    /// Currently idle
    Idle,
    /// A command is in progress (sent to the server but not yet completed)
    Active,
    /// Idle, in a valid transaction block
    InTransaction,
    /// Idle, in a failed transaction block
    InError,
    /// Bad connection
    Unknown,
}

impl From<PGTransactionStatusType> for PgTransactionStatus {
    fn from(trans_status_type: PGTransactionStatusType) -> Self {
        match trans_status_type {
            PGTransactionStatusType::PQTRANS_IDLE => PgTransactionStatus::Idle,
            PGTransactionStatusType::PQTRANS_ACTIVE => PgTransactionStatus::Active,
            PGTransactionStatusType::PQTRANS_INTRANS => PgTransactionStatus::InTransaction,
            PGTransactionStatusType::PQTRANS_INERROR => PgTransactionStatus::InError,
            PGTransactionStatusType::PQTRANS_UNKNOWN => PgTransactionStatus::Unknown,
        }
    }
}

pub(super) type NoticeProcessor =
    extern "C" fn(arg: *mut libc::c_void, message: *const libc::c_char);

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
pub(super) struct RawResult(NonNull<PGresult>);

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

    pub(super) fn as_ptr(&self) -> *mut PGresult {
        self.0.as_ptr()
    }

    pub(super) fn error_message(&self) -> &str {
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
