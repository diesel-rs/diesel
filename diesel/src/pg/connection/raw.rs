#![allow(clippy::too_many_arguments)]

extern crate pq_sys;

use self::pq_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{ptr, str};
use tokio::io::Registration;

use crate::result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    internal_connection: NonNull<PGconn>,
}

unsafe impl Send for RawConnection {}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        use self::ConnStatusType::*;

        let connection_string = CString::new(database_url)?;
        let connection_ptr = unsafe { PQconnectdb(connection_string.as_ptr()) };
        let connection_status = unsafe { PQstatus(connection_ptr) };

        match connection_status {
            CONNECTION_OK => {
                let connection_ptr = unsafe { NonNull::new_unchecked(connection_ptr) };
                Ok(RawConnection { internal_connection: connection_ptr })
            }
            _ => {
                let message = last_error_message(connection_ptr);
                Err(ConnectionError::BadConnection(message))
            }
        }
    }

    pub async fn establish_async(database_url: &str) -> ConnectionResult<Self> {
        use futures::future::poll_fn;
        use self::PostgresPollingStatusType::*;

        let connection_string = CString::new(database_url)?;
        let connection_ptr = unsafe { PQconnectStart(connection_string.as_ptr()) };
        let connection_ptr = NonNull::new(connection_ptr)
            .ok_or_else(|| ConnectionError::BadConnection("Out of memory".into()))?;

        let mut state = PGRES_POLLING_WRITING;
        loop {
            let socket = unsafe { PQsocket(connection_ptr.as_ptr()) };
            let evented_fd = mio::unix::EventedFd(&socket);
            let registration = Registration::new(&evented_fd)?;

            match state {
                PGRES_POLLING_FAILED => {
                    return Err(ConnectionError::BadConnection(last_error_message(connection_ptr.as_ptr())));
                },
                PGRES_POLLING_READING => {
                    poll_fn(|cx| registration.poll_read_ready(cx)).await?;
                    state = unsafe { PQconnectPoll(connection_ptr.as_ptr()) };
                }
                PGRES_POLLING_WRITING => {
                    poll_fn(|cx| registration.poll_write_ready(cx)).await?;
                    state = unsafe { PQconnectPoll(connection_ptr.as_ptr()) };
                }
                PGRES_POLLING_OK => return Ok(Self { internal_connection: connection_ptr }),
                PGRES_POLLING_ACTIVE => {
                    // Claimed by
                    // https://www.postgresql.org/message-id/20030226163108.GA1355@gnu.org. In any
                    // case the value is undocumented and there's no obvious action to take here.
                    unreachable!("PGRES_POLLING_ACTIVE is unused")
                }
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

    pub unsafe fn exec_prepared_async(
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
        if res == 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
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

    pub unsafe fn prepare_async(
        &mut self,
        stmt_name: *const libc::c_char,
        query: *const libc::c_char,
        param_count: libc::c_int,
        param_types: *const Oid,
    ) -> QueryResult<()> {
        let res = PQsendPrepare(
            self.internal_connection.as_ptr(),
            stmt_name,
            query,
            param_count,
            param_types,
        );

        if res == 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    pub(crate) fn set_nonblocking(&self) -> ConnectionResult<()> {
        let res = unsafe {
            PQsetnonblocking(self.internal_connection.as_ptr(), 1)
        };
        if res == 0 {
            Ok(())
        } else {
            Err(ConnectionError::BadConnection(self.last_error_message()))
        }
    }

    pub async fn get_last_result(&mut self) -> QueryResult<RawResult> {
        use futures::future::poll_fn;
        let mut needs_write = self.flush()?;
        let socket = unsafe { PQsocket(self.internal_connection.as_ptr()) };
        let evented_fd = mio::unix::EventedFd(&socket);
        let registration = Registration::new(&evented_fd)
            // FIXME
            .unwrap();

        let mut result = None;
        loop {
            // Not waiting on writes, and `PQgetResult` will not block.
            while !needs_write && !self.is_busy() {
                let ptr = unsafe { PQgetResult(self.internal_connection.as_ptr()) };
                if ptr.is_null() {
                    return result.ok_or_else(|| self.last_error());
                } else {
                    result = Some(RawResult::new(ptr, self)?);
                }
            }

            // Ok, let's wait for read/write readiness
            let ready = poll_fn(|cx| {
                let read_ready = registration.poll_read_ready(cx);
                if read_ready.is_pending() && needs_write {
                    registration.poll_write_ready(cx)
                } else {
                    read_ready
                }
            }).await
            // FIXME
            .unwrap();

            if ready.is_readable() {
                self.consume_input()?;
            } else if needs_write {
                self.flush()?;
            } else {
                unreachable!("Got non-readable readiness when not writing");
            }
        }
    }

    /// Attempts to flush any queued output data to the server.
    ///
    /// If this function returns `true`, it must be called again after a write
    /// readiness
    fn flush(&mut self) -> QueryResult<bool> {
        let result = unsafe { PQflush(self.internal_connection.as_ptr()) };
        if result == -1 {
            Err(self.last_error())
        } else {
            Ok(result != 0)
        }
    }

    fn consume_input(&mut self) -> QueryResult<()> {
        let res = unsafe { PQconsumeInput(self.internal_connection.as_ptr()) };
        if res != 1 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    fn last_error(&self) -> Error {
        Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new(self.last_error_message()),
        )
    }

    fn is_busy(&self) -> bool {
        let res = unsafe { PQisBusy(self.internal_connection.as_ptr()) };
        res != 0
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
        str::from_utf8_unchecked(bytes).to_string()
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
            conn.last_error()
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
