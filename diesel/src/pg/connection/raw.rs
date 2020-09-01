#![allow(clippy::too_many_arguments)]

extern crate pq_sys;

use self::pq_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{ptr, str};
use std::future::Future;
use std::task::*;

use crate::result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    internal_connection: NonNull<PGconn>,
}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        use self::ConnStatusType::*;

        let connection_string = CString::new(database_url)?;
        let connection_ptr = unsafe { PQconnectdb(connection_string.as_ptr()) };
        let connection_status = unsafe { PQstatus(connection_ptr) };

        match connection_status {
            CONNECTION_OK => {
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
        self.command_result(RawResult::new(PQexec(self.internal_connection.as_ptr(), query)))
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
        self.command_result(RawResult::new(ptr))
    }

    pub async unsafe fn prepare(
        &mut self,
        stmt_name: *const libc::c_char,
        query: *const libc::c_char,
        param_count: libc::c_int,
        param_types: *const Oid,
    ) -> QueryResult<RawResult> {
        let success = PQsendPrepare(
            self.internal_connection.as_ptr(),
            stmt_name,
            query,
            param_count,
            param_types,
        );

        if success == 0 {
            return Err(self.unable_to_send_command());
        }

        self.last_pending_result().await
    }

    /// Run the given function and block on the result on this connection's
    /// executor. If this connection is blocking, the given future must never
    /// yield. Panics if the future yields with a blocking connection
    pub fn block_on<'a, Func, Fut>(&'a mut self, f: Func) -> Fut::Output
    where
        Func: FnOnce(&'a mut Self) -> Fut,
        Fut: Future,
    {
        // This is only for when we're compiled without async support, and we
        // know statically that we the futures never yield. I'm not 100% sure
        // what we want to do if this is called when tokio is enabled, I think
        // we'll want to consider that a bug.
        //
        // This is used instead of `futures::block_on` to limit compile times,
        // and also because we don't need the overhead of some of the checks it
        // performs. We maybe want to do some proper park/unparking here but
        // TBH if we're going that far we should use a better implemented
        // version, and I think we can just straight up assume that user defined
        // futures never make it here
        use std::pin::Pin;

        let waker = unsafe { Waker::from_raw(noop_waker()) };
        let mut context = Context::from_waker(&waker);
        let mut future = f(self);
        let pinned = unsafe { Pin::new_unchecked(&mut future) };
        match pinned.poll(&mut context) {
            Poll::Ready(x) => x,
            Poll::Pending => panic!("blocking connection yielded"),
        }
    }

    async fn last_pending_result(&mut self) -> QueryResult<RawResult> {
        let mut last_result = None;
        loop {
            self.wait_for_idle_if_non_blocking().await;
            let result = self.get_result();
            if result.is_none() {
                break;
            }
            last_result = result;
        }

        self.command_result(last_result)
    }

    #[cfg(not(feature = "async"))]
    /// Returns immediately, Diesel was compiled without async support
    async fn wait_for_idle_if_non_blocking(&mut self) {
        // For non blocking connections this struct is going to have a
        // `tokio::io::Registration`, so if the lib is compiled with async
        // support this function will check if that field is `Some` and do
        // the appropriate read/write readiness checks
    }

    /// Get the next pending result. Blocks unless
    /// `wait_for_idle_if_non_blocking` has resolved.
    fn get_result(&self) -> Option<RawResult> {
        RawResult::new(unsafe { PQgetResult(self.internal_connection.as_ptr()) })
    }

    fn command_result(&self, result: Option<RawResult>) -> QueryResult<RawResult> {
        result.ok_or_else(|| self.unable_to_send_command())
    }

    fn unable_to_send_command(&self) -> Error {
        Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new(self.last_error_message()),
        )
    }
}

fn noop_waker() -> RawWaker {
    fn noop_clone(_: *const ()) -> RawWaker {
        noop_waker()
    }
    fn noop_wake(_: *const ()) {}
    fn noop_wake_by_ref(_: *const ()) {}
    fn noop_drop(_: *const ()) {}

    RawWaker::new(
        ptr::null(),
        &RawWakerVTable::new(
            noop_clone,
            noop_wake,
            noop_wake_by_ref,
            noop_drop,
        ),
    )
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
    fn new(ptr: *mut PGresult) -> Option<Self> {
        NonNull::new(ptr).map(RawResult)
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
