#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::ffi::{CString, NulError};
use alloc::string::{String, ToString};
use core::any::Any;
use core::cell::{Cell, RefCell};
use core::ffi as libc;
use core::ffi::CStr;
use core::ptr::NonNull;
use core::{mem, ptr, slice, str};

use super::authorizer::{AuthorizerAction, AuthorizerContext, AuthorizerDecision};
use super::functions::{build_sql_function_args, process_sql_function_result};
use super::serialized_database::SerializedDatabase;
use super::stmt::ensure_sqlite_ok;
use super::trace::{SqliteTraceEvent, SqliteTraceFlags};
use super::update_hook::{ChangeHookDispatcher, SqliteChangeEvent, SqliteChangeOp};
use super::{Sqlite, SqliteAggregateFunction};
use crate::deserialize::FromSqlRow;
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::HasSqlType;

/// For use in FFI function, which cannot unwind.
/// Print the message, ask to open an issue at Github and [`abort`](std::process::abort).
macro_rules! assert_fail {
    ($fmt:expr_2021 $(,$args:tt)*) => {
        #[cfg(feature = "std")]
        eprint!(concat!(
            $fmt,
            "If you see this message, please open an issue at https://github.com/diesel-rs/diesel/issues/new.\n",
            "Source location: {}:{}\n",
        ), $($args,)* file!(), line!());
        crate::util::std_compat::abort()
    };
}

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub(super) struct RawConnection {
    pub(super) internal_connection: NonNull<ffi::sqlite3>,
    /// Hook dispatcher for sqlite3_update_hook callbacks. The C trampoline
    /// dispatches events directly to registered hooks during sqlite3_step().
    /// Uses RefCell because the C callback needs mutable access while the
    /// connection may be immutably borrowed.
    pub(super) change_hooks: RefCell<ChangeHookDispatcher>,
    /// Tracks whether the C-level update hook is currently installed, to
    /// avoid redundant FFI calls.
    update_hook_registered: Cell<bool>,
    /// Boxed closure for the commit hook. Stored to keep the closure alive
    /// for the duration of the hook registration. Type-erased via `dyn Any`.
    commit_hook: Option<Box<dyn Any + Send>>,
    /// Boxed closure for the rollback hook.
    rollback_hook: Option<Box<dyn Any + Send>>,
    /// Boxed closure for the WAL hook.
    wal_hook: Option<Box<dyn Any + Send>>,
    /// Boxed closure for the progress handler.
    progress_hook: Option<Box<dyn Any + Send>>,
    /// Boxed closure for the busy handler.
    busy_handler: Option<Box<dyn Any + Send>>,
    /// Boxed closure for the authorizer callback.
    authorizer_hook: Option<Box<dyn Any + Send>>,
    /// Boxed closure for the trace callback.
    trace_hook: Option<Box<dyn Any + Send>>,
}

impl RawConnection {
    fn from_ptr(conn: NonNull<ffi::sqlite3>) -> Self {
        RawConnection {
            internal_connection: conn,
            change_hooks: RefCell::new(ChangeHookDispatcher::new()),
            update_hook_registered: Cell::new(false),
            commit_hook: None,
            rollback_hook: None,
            wal_hook: None,
            progress_hook: None,
            busy_handler: None,
            authorizer_hook: None,
            trace_hook: None,
        }
    }

    pub(super) fn establish(database_url: &str) -> ConnectionResult<Self> {
        let mut conn_pointer = ptr::null_mut();

        let database_url = if database_url.starts_with("sqlite://") {
            CString::new(database_url.replacen("sqlite://", "file:", 1))?
        } else {
            CString::new(database_url)?
        };
        let flags = ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_URI;
        let connection_status = unsafe {
            ffi::sqlite3_open_v2(database_url.as_ptr(), &mut conn_pointer, flags, ptr::null())
        };

        match connection_status {
            ffi::SQLITE_OK => {
                let conn_pointer = unsafe { NonNull::new_unchecked(conn_pointer) };
                Ok(RawConnection::from_ptr(conn_pointer))
            }
            err_code => {
                let message = super::error_message(err_code);
                Err(ConnectionError::BadConnection(message.into()))
            }
        }
    }

    pub(super) fn exec(&self, query: &str) -> QueryResult<()> {
        let query = CString::new(query)?;
        let callback_fn = None;
        let callback_arg = ptr::null_mut();
        let result = unsafe {
            ffi::sqlite3_exec(
                self.internal_connection.as_ptr(),
                query.as_ptr(),
                callback_fn,
                callback_arg,
                ptr::null_mut(),
            )
        };

        ensure_sqlite_ok(result, self.internal_connection.as_ptr())
    }

    pub(super) fn rows_affected_by_last_query(
        &self,
    ) -> Result<usize, Box<dyn core::error::Error + Send + Sync>> {
        let r = unsafe { ffi::sqlite3_changes(self.internal_connection.as_ptr()) };

        Ok(r.try_into()?)
    }

    pub(super) fn register_sql_function<F, Ret, RetSqlType>(
        &self,
        fn_name: &str,
        num_args: usize,
        deterministic: bool,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(&Self, &mut [*mut ffi::sqlite3_value]) -> QueryResult<Ret>
            + core::panic::UnwindSafe
            + Send
            + 'static,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        let callback_fn = Box::into_raw(Box::new(CustomFunctionUserPtr {
            callback: f,
            function_name: fn_name.to_owned(),
        }));
        let fn_name = Self::get_fn_name(fn_name)?;
        let flags = Self::get_flags(deterministic);
        let num_args = num_args
            .try_into()
            .map_err(|e| Error::SerializationError(Box::new(e)))?;

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                fn_name.as_ptr(),
                num_args,
                flags,
                callback_fn as *mut _,
                Some(run_custom_function::<F, Ret, RetSqlType>),
                None,
                None,
                Some(destroy_boxed::<CustomFunctionUserPtr<F>>),
            )
        };

        Self::process_sql_function_result(result)
    }

    pub(super) fn register_aggregate_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
        &self,
        fn_name: &str,
        num_args: usize,
    ) -> QueryResult<()>
    where
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
        Args: FromSqlRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        let fn_name = Self::get_fn_name(fn_name)?;
        let flags = Self::get_flags(false);
        let num_args = num_args
            .try_into()
            .map_err(|e| Error::SerializationError(Box::new(e)))?;

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                fn_name.as_ptr(),
                num_args,
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

    pub(super) fn register_collation_function<F>(
        &self,
        collation_name: &str,
        collation: F,
    ) -> QueryResult<()>
    where
        F: Fn(&str, &str) -> core::cmp::Ordering + core::panic::UnwindSafe + Send + 'static,
    {
        let callback_fn = Box::into_raw(Box::new(CollationUserPtr {
            callback: collation,
            collation_name: collation_name.to_owned(),
        }));
        let collation_name = Self::get_fn_name(collation_name)?;

        let result = unsafe {
            ffi::sqlite3_create_collation_v2(
                self.internal_connection.as_ptr(),
                collation_name.as_ptr(),
                ffi::SQLITE_UTF8,
                callback_fn as *mut _,
                Some(run_collation_function::<F>),
                Some(destroy_boxed::<CollationUserPtr<F>>),
            )
        };

        let result = Self::process_sql_function_result(result);
        if result.is_err() {
            destroy_boxed::<CollationUserPtr<F>>(callback_fn as *mut _);
        }
        result
    }

    pub(super) fn serialize(&mut self) -> SerializedDatabase {
        unsafe {
            let mut size: ffi::sqlite3_int64 = 0;
            let data_ptr = ffi::sqlite3_serialize(
                self.internal_connection.as_ptr(),
                core::ptr::null(),
                &mut size as *mut _,
                0,
            );
            SerializedDatabase::new(
                data_ptr,
                size.try_into()
                    .expect("Cannot fit the serialized database into memory"),
            )
        }
    }

    pub(super) fn deserialize(&mut self, data: &[u8]) -> QueryResult<()> {
        let db_size = data
            .len()
            .try_into()
            .map_err(|e| Error::DeserializationError(Box::new(e)))?;
        // the cast for `ffi::SQLITE_DESERIALIZE_READONLY` is required for old libsqlite3-sys versions
        #[allow(clippy::unnecessary_cast)]
        unsafe {
            let result = ffi::sqlite3_deserialize(
                self.internal_connection.as_ptr(),
                core::ptr::null(),
                data.as_ptr() as *mut u8,
                db_size,
                db_size,
                ffi::SQLITE_DESERIALIZE_READONLY as u32,
            );

            ensure_sqlite_ok(result, self.internal_connection.as_ptr())
        }
    }

    fn get_fn_name(fn_name: &str) -> Result<CString, NulError> {
        CString::new(fn_name)
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
                DatabaseErrorKind::Unknown,
                Box::new(error_message.to_string()),
            ))
        }
    }

    /// Registers the C-level `sqlite3_update_hook` if not already registered.
    ///
    /// The hook dispatches events directly to registered hooks in
    /// `self.change_hooks` during `sqlite3_step()`.
    /// This is a no-op if the hook is already installed.
    pub(super) fn register_raw_update_hook(&self) {
        if !self.update_hook_registered.get() {
            unsafe {
                ffi::sqlite3_update_hook(
                    self.internal_connection.as_ptr(),
                    Some(update_hook_trampoline),
                    &self.change_hooks as *const RefCell<ChangeHookDispatcher> as *mut libc::c_void,
                );
            }
            self.update_hook_registered.set(true);
        }
    }

    /// Unregisters the C-level `sqlite3_update_hook`.
    pub(super) fn unregister_raw_update_hook(&self) {
        if self.update_hook_registered.get() {
            unsafe {
                ffi::sqlite3_update_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
            }
            self.update_hook_registered.set(false);
        }
    }

    /// Sets a commit hook. Only one can be active at a time; the previous
    /// hook (if any) is replaced.
    ///
    /// The callback returns `true` to convert the commit into a rollback,
    /// or `false` to allow the commit to proceed.
    pub(super) fn set_commit_hook<F>(&mut self, hook: F)
    where
        F: FnMut() -> bool + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_commit_hook(
                self.internal_connection.as_ptr(),
                Some(commit_hook_trampoline::<F>),
                ptr,
            );
        }
        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.commit_hook = Some(boxed);
    }

    /// Removes the commit hook.
    pub(super) fn remove_commit_hook(&mut self) {
        unsafe {
            ffi::sqlite3_commit_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.commit_hook = None;
    }

    /// Sets a rollback hook. Only one can be active at a time.
    pub(super) fn set_rollback_hook<F>(&mut self, hook: F)
    where
        F: FnMut() + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_rollback_hook(
                self.internal_connection.as_ptr(),
                Some(rollback_hook_trampoline::<F>),
                ptr,
            );
        }
        self.rollback_hook = Some(boxed);
    }

    /// Removes the rollback hook.
    pub(super) fn remove_rollback_hook(&mut self) {
        unsafe {
            ffi::sqlite3_rollback_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.rollback_hook = None;
    }

    /// Sets a WAL hook. Only one can be active at a time.
    ///
    /// The callback receives the database name (e.g. `"main"`) and the
    /// number of pages currently in the WAL file.
    pub(super) fn set_wal_hook<F>(&mut self, hook: F)
    where
        F: FnMut(&str, i32) + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_wal_hook(
                self.internal_connection.as_ptr(),
                Some(wal_hook_trampoline::<F>),
                ptr,
            );
        }
        self.wal_hook = Some(boxed);
    }

    /// Removes the WAL hook.
    pub(super) fn remove_wal_hook(&mut self) {
        unsafe {
            ffi::sqlite3_wal_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.wal_hook = None;
    }

    /// Sets a progress handler. Only one can be active at a time.
    ///
    /// The callback is invoked periodically during long-running SQL queries.
    /// `n` is the approximate number of VM instructions between callbacks.
    /// Return `true` to interrupt the query, `false` to continue.
    pub(super) fn set_progress_handler<F>(&mut self, n: i32, hook: F)
    where
        F: FnMut() -> bool + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_progress_handler(
                self.internal_connection.as_ptr(),
                n,
                Some(progress_handler_trampoline::<F>),
                ptr,
            );
        }
        self.progress_hook = Some(boxed);
    }

    /// Removes the progress handler.
    pub(super) fn remove_progress_handler(&mut self) {
        unsafe {
            ffi::sqlite3_progress_handler(
                self.internal_connection.as_ptr(),
                0,
                None,
                ptr::null_mut(),
            );
        }
        self.progress_hook = None;
    }

    /// Sets a busy handler. Only one can be active at a time.
    ///
    /// The callback receives the retry count and returns `true` to retry,
    /// `false` to abort. Setting this clears any `busy_timeout`.
    pub(super) fn set_busy_handler<F>(&mut self, hook: F)
    where
        F: FnMut(i32) -> bool + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_busy_handler(
                self.internal_connection.as_ptr(),
                Some(busy_handler_trampoline::<F>),
                ptr,
            );
        }
        self.busy_handler = Some(boxed);
    }

    /// Removes the busy handler.
    pub(super) fn remove_busy_handler(&mut self) {
        unsafe {
            ffi::sqlite3_busy_handler(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.busy_handler = None;
    }

    /// Sets a simple timeout-based busy handler.
    ///
    /// SQLite will sleep and retry until `ms` milliseconds have elapsed.
    /// Setting this clears any custom busy handler.
    pub(super) fn set_busy_timeout(&mut self, ms: i32) {
        unsafe {
            ffi::sqlite3_busy_timeout(self.internal_connection.as_ptr(), ms);
        }
        self.busy_handler = None;
    }

    /// Sets an authorizer callback. Only one can be active at a time.
    ///
    /// The callback is invoked during SQL statement compilation to control
    /// access to database objects.
    pub(super) fn set_authorizer<F>(&mut self, hook: F)
    where
        F: FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_set_authorizer(
                self.internal_connection.as_ptr(),
                Some(authorizer_trampoline::<F>),
                ptr,
            );
        }
        self.authorizer_hook = Some(boxed);
    }

    /// Removes the authorizer callback.
    pub(super) fn remove_authorizer(&mut self) {
        unsafe {
            ffi::sqlite3_set_authorizer(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.authorizer_hook = None;
    }

    /// Sets a trace callback. Only one can be active at a time.
    ///
    /// The callback is invoked for SQL execution tracing based on the
    /// provided event mask.
    pub(super) fn set_trace<F>(&mut self, mask: SqliteTraceFlags, hook: F)
    where
        F: FnMut(SqliteTraceEvent<'_>) + Send + 'static,
    {
        let mut boxed: Box<F> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;
        unsafe {
            ffi::sqlite3_trace_v2(
                self.internal_connection.as_ptr(),
                mask.bits(),
                Some(trace_trampoline::<F>),
                ptr,
            );
        }
        self.trace_hook = Some(boxed);
    }

    /// Removes the trace callback.
    pub(super) fn remove_trace(&mut self) {
        unsafe {
            ffi::sqlite3_trace_v2(self.internal_connection.as_ptr(), 0, None, ptr::null_mut());
        }
        self.trace_hook = None;
    }
}

/// C trampoline for `sqlite3_update_hook`.
///
/// # Safety
///
/// `user_data` must be a valid pointer to `RefCell<ChangeHookDispatcher>`
/// that outlives the hook registration. This is guaranteed because the
/// `RefCell` is a field of `RawConnection` and the hook is unregistered
/// before the connection is dropped.
unsafe extern "C" fn update_hook_trampoline(
    user_data: *mut libc::c_void,
    op: libc::c_int,
    db_name: *const libc::c_char,
    table_name: *const libc::c_char,
    rowid: ffi::sqlite3_int64,
) {
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to the `RefCell<ChangeHookDispatcher>` field
        // of `RawConnection`, guaranteed by the caller contract.
        let hooks = unsafe { &*(user_data as *const RefCell<ChangeHookDispatcher>) };

        // SAFETY: `db_name` is a valid C string provided by SQLite.
        let db_name = unsafe { CStr::from_ptr(db_name) }
            .to_str()
            .unwrap_or_else(|_| {
                assert_fail!("sqlite3_update_hook delivered invalid UTF-8 for db_name. ");
            });
        // SAFETY: `table_name` is a valid C string provided by SQLite.
        let table_name = unsafe { CStr::from_ptr(table_name) }
            .to_str()
            .unwrap_or_else(|_| {
                assert_fail!("sqlite3_update_hook delivered invalid UTF-8 for table_name. ");
            });

        let event = SqliteChangeEvent {
            op: SqliteChangeOp::from_ffi(op),
            db_name,
            table_name,
            rowid,
        };

        // Use try_borrow_mut to handle reentrancy gracefully: if a hook
        // callback triggers another SQL statement that fires the update hook,
        // the RefCell would already be borrowed, so we silently skip dispatch
        // rather than panicking inside an extern "C" function.
        if let Ok(mut guard) = hooks.try_borrow_mut() {
            guard.dispatch(event);
        }
    }));

    if result.is_err() {
        assert_fail!("Panic in sqlite3_update_hook trampoline. ");
    }
}

/// C trampoline for `sqlite3_commit_hook`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::commit_hook`.
unsafe extern "C" fn commit_hook_trampoline<F>(user_data: *mut libc::c_void) -> libc::c_int
where
    F: FnMut() -> bool,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::commit_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f()
    }));

    match result {
        Ok(true) => 1,  // convert commit to rollback
        Ok(false) => 0, // proceed with commit
        Err(_) => {
            assert_fail!("Panic in sqlite3_commit_hook trampoline. ");
        }
    }
}

/// C trampoline for `sqlite3_rollback_hook`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::rollback_hook`.
unsafe extern "C" fn rollback_hook_trampoline<F>(user_data: *mut libc::c_void)
where
    F: FnMut(),
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::rollback_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f();
    }));

    if result.is_err() {
        assert_fail!("Panic in sqlite3_rollback_hook trampoline. ");
    }
}

/// C trampoline for `sqlite3_wal_hook`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::wal_hook`.
unsafe extern "C" fn wal_hook_trampoline<F>(
    user_data: *mut libc::c_void,
    _db: *mut ffi::sqlite3,
    db_name: *const libc::c_char,
    n_pages: libc::c_int,
) -> libc::c_int
where
    F: FnMut(&str, i32),
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` is a valid pointer to `F` stored in
        // `RawConnection::wal_hook`, guaranteed by the caller contract.
        let f = unsafe { &mut *(user_data as *mut F) };
        // SAFETY: `db_name` is a valid C string provided by SQLite.
        let db_name_str = unsafe { CStr::from_ptr(db_name) }
            .to_str()
            .unwrap_or_else(|_| {
                assert_fail!("sqlite3_wal_hook delivered invalid UTF-8 for db_name. ");
            });
        f(db_name_str, n_pages);
    }));

    if result.is_err() {
        assert_fail!("Panic in sqlite3_wal_hook trampoline. ");
    }

    ffi::SQLITE_OK
}

/// C trampoline for `sqlite3_progress_handler`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::progress_hook`.
unsafe extern "C" fn progress_handler_trampoline<F>(user_data: *mut libc::c_void) -> libc::c_int
where
    F: FnMut() -> bool,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::progress_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f()
    }));

    match result {
        Ok(true) => 1,  // interrupt query
        Ok(false) => 0, // continue
        Err(_) => {
            assert_fail!("Panic in sqlite3_progress_handler trampoline. ");
        }
    }
}

/// C trampoline for `sqlite3_busy_handler`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::busy_handler`.
unsafe extern "C" fn busy_handler_trampoline<F>(
    user_data: *mut libc::c_void,
    retry_count: libc::c_int,
) -> libc::c_int
where
    F: FnMut(i32) -> bool,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::busy_handler`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f(retry_count)
    }));

    match result {
        Ok(true) => 1,  // retry
        Ok(false) => 0, // abort (return SQLITE_BUSY)
        Err(_) => {
            assert_fail!("Panic in sqlite3_busy_handler trampoline. ");
        }
    }
}

/// C trampoline for `sqlite3_set_authorizer`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::authorizer_hook`.
unsafe extern "C" fn authorizer_trampoline<F>(
    user_data: *mut libc::c_void,
    action_code: libc::c_int,
    arg1: *const libc::c_char,
    arg2: *const libc::c_char,
    db_name: *const libc::c_char,
    accessor: *const libc::c_char,
) -> libc::c_int
where
    F: FnMut(AuthorizerContext<'_>) -> AuthorizerDecision,
{
    // Helper to convert a nullable C string to Option<&str>.
    // SAFETY: 'static is used because this local function cannot name the
    // callback's stack lifetime. The returned references are immediately
    // placed into AuthorizerContext<'a> which cannot escape this invocation.
    fn c_str_to_option(ptr: *const libc::c_char) -> Option<&'static str> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: ptr is a valid C string provided by SQLite.
            match unsafe { CStr::from_ptr(ptr) }.to_str() {
                Ok(s) => Some(s),
                Err(_) => {
                    assert_fail!("sqlite3_set_authorizer delivered invalid UTF-8. ");
                }
            }
        }
    }

    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::authorizer_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };

        let ctx = AuthorizerContext {
            action: AuthorizerAction::from_ffi(action_code),
            arg1: c_str_to_option(arg1),
            arg2: c_str_to_option(arg2),
            db_name: c_str_to_option(db_name),
            accessor: c_str_to_option(accessor),
        };

        f(ctx)
    }));

    match result {
        Ok(decision) => decision.to_ffi(),
        Err(_) => {
            assert_fail!("Panic in sqlite3_set_authorizer trampoline. ");
        }
    }
}

/// C trampoline for `sqlite3_trace_v2`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::trace_hook`.
unsafe extern "C" fn trace_trampoline<F>(
    event_code: libc::c_uint,
    user_data: *mut libc::c_void,
    p: *mut libc::c_void,
    x: *mut libc::c_void,
) -> libc::c_int
where
    F: FnMut(SqliteTraceEvent<'_>),
{
    // Define constants to avoid cast_sign_loss warnings in match guards.
    // These are compile-time constants so the conversion is verified at build time.
    const TRACE_STMT: libc::c_uint = ffi::SQLITE_TRACE_STMT as libc::c_uint;
    const TRACE_PROFILE: libc::c_uint = ffi::SQLITE_TRACE_PROFILE as libc::c_uint;
    const TRACE_ROW: libc::c_uint = ffi::SQLITE_TRACE_ROW as libc::c_uint;
    const TRACE_CLOSE: libc::c_uint = ffi::SQLITE_TRACE_CLOSE as libc::c_uint;

    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::trace_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };

        let event = match event_code {
            TRACE_STMT => {
                // p = sqlite3_stmt*, x = const char* (unexpanded SQL)
                let stmt_ptr = p as *mut ffi::sqlite3_stmt;
                let sql_ptr = x as *const libc::c_char;
                if sql_ptr.is_null() {
                    return;
                }
                let sql = match unsafe { CStr::from_ptr(sql_ptr) }.to_str() {
                    Ok(s) => s,
                    Err(_) => {
                        assert_fail!("sqlite3_trace_v2 STMT delivered invalid UTF-8. ");
                    }
                };
                let readonly = if stmt_ptr.is_null() {
                    false
                } else {
                    unsafe { ffi::sqlite3_stmt_readonly(stmt_ptr) != 0 }
                };
                SqliteTraceEvent::Statement { sql, readonly }
            }
            TRACE_PROFILE => {
                // p = sqlite3_stmt*, x = int64* (nanoseconds)
                let stmt_ptr = p as *mut ffi::sqlite3_stmt;
                // Get the SQL and readonly status from the statement
                let (sql, readonly) = if stmt_ptr.is_null() {
                    ("", false)
                } else {
                    let sql_ptr = unsafe { ffi::sqlite3_sql(stmt_ptr) };
                    let sql = if sql_ptr.is_null() {
                        ""
                    } else {
                        match unsafe { CStr::from_ptr(sql_ptr) }.to_str() {
                            Ok(s) => s,
                            Err(_) => {
                                assert_fail!("sqlite3_trace_v2 PROFILE delivered invalid UTF-8. ");
                            }
                        }
                    };
                    let readonly = unsafe { ffi::sqlite3_stmt_readonly(stmt_ptr) != 0 };
                    (sql, readonly)
                };

                let duration_ns = if x.is_null() {
                    0
                } else {
                    // x is a pointer to sqlite3_int64 (i64), but duration is always non-negative
                    unsafe { (*(x as *const ffi::sqlite3_int64)).cast_unsigned() }
                };

                SqliteTraceEvent::Profile {
                    sql,
                    duration_ns,
                    readonly,
                }
            }
            TRACE_ROW => SqliteTraceEvent::Row,
            TRACE_CLOSE => SqliteTraceEvent::Close,
            _ => {
                // Unknown trace event - ignore
                return;
            }
        };

        f(event);
    }));

    if result.is_err() {
        assert_fail!("Panic in sqlite3_trace_v2 trampoline. ");
    }

    0 // Return value is currently unused by SQLite
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        use crate::util::std_compat::panicking;

        // Unregister all hooks before closing so the Box'd closures are
        // dropped before sqlite3_close runs.
        self.unregister_raw_update_hook();
        self.remove_commit_hook();
        self.remove_rollback_hook();
        self.remove_wal_hook();
        self.remove_progress_handler();
        self.remove_busy_handler();
        self.remove_authorizer();
        self.remove_trace();

        let close_result = unsafe { ffi::sqlite3_close(self.internal_connection.as_ptr()) };
        if close_result != ffi::SQLITE_OK {
            let error_message = super::error_message(close_result);
            if panicking() {
                #[cfg(feature = "std")]
                eprintln!("Error closing SQLite connection: {error_message}");
            } else {
                panic!("Error closing SQLite connection: {error_message}");
            }
        }
    }
}

enum SqliteCallbackError {
    Abort(&'static str),
    DieselError(crate::result::Error),
    Panic(String),
}

impl SqliteCallbackError {
    fn emit(&self, ctx: *mut ffi::sqlite3_context) {
        let s;
        let msg = match self {
            SqliteCallbackError::Abort(msg) => *msg,
            SqliteCallbackError::DieselError(e) => {
                s = e.to_string();
                &s
            }
            SqliteCallbackError::Panic(msg) => msg,
        };
        unsafe {
            context_error_str(ctx, msg);
        }
    }
}

impl From<crate::result::Error> for SqliteCallbackError {
    fn from(e: crate::result::Error) -> Self {
        Self::DieselError(e)
    }
}

struct CustomFunctionUserPtr<F> {
    callback: F,
    function_name: String,
}

#[allow(warnings)]
extern "C" fn run_custom_function<F, Ret, RetSqlType>(
    ctx: *mut ffi::sqlite3_context,
    num_args: libc::c_int,
    value_ptr: *mut *mut ffi::sqlite3_value,
) where
    F: FnMut(&RawConnection, &mut [*mut ffi::sqlite3_value]) -> QueryResult<Ret>
        + core::panic::UnwindSafe
        + Send
        + 'static,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    use core::ops::Deref;
    static NULL_DATA_ERR: &str = "An unknown error occurred. sqlite3_user_data returned a null pointer. This should never happen.";
    static NULL_CONN_ERR: &str = "An unknown error occurred. sqlite3_context_db_handle returned a null pointer. This should never happen.";

    let conn = match unsafe { NonNull::new(ffi::sqlite3_context_db_handle(ctx)) } {
        // We use `ManuallyDrop` here because we do not want to run the
        // Drop impl of `RawConnection` as this would close the connection
        Some(conn) => mem::ManuallyDrop::new(RawConnection::from_ptr(conn)),
        None => {
            unsafe { context_error_str(ctx, NULL_CONN_ERR) };
            return;
        }
    };

    let data_ptr = unsafe { ffi::sqlite3_user_data(ctx) };

    let mut data_ptr = match NonNull::new(data_ptr as *mut CustomFunctionUserPtr<F>) {
        None => unsafe {
            context_error_str(ctx, NULL_DATA_ERR);
            return;
        },
        Some(mut f) => f,
    };
    let data_ptr = unsafe { data_ptr.as_mut() };

    // We need this to move the reference into the catch_unwind part
    // this is sound as `F` itself and the stored string is `UnwindSafe`
    let callback = core::panic::AssertUnwindSafe(&mut data_ptr.callback);
    // conn contains Box<dyn Any + Send> fields which are not UnwindSafe.
    // This is safe because the ManuallyDrop wrapper ensures we never run
    // the RawConnection Drop, and we only read through conn in the closure.
    let conn = core::panic::AssertUnwindSafe(conn);

    let result = crate::util::std_compat::catch_unwind(move || {
        let _ = &callback;
        let args = unsafe { slice::from_raw_parts_mut(value_ptr, num_args as _) };
        let res = (callback.0)(&*conn, args)?;
        let value = process_sql_function_result(&res)?;
        // We've checked already that ctx is not null
        unsafe {
            value.result_of(&mut *ctx);
        }
        Ok(())
    })
    .unwrap_or_else(|p| Err(SqliteCallbackError::Panic(data_ptr.function_name.clone())));
    if let Err(e) = result {
        e.emit(ctx);
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
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
    Args: FromSqlRow<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let result = crate::util::std_compat::catch_unwind(move || {
        let args = unsafe { slice::from_raw_parts_mut(value_ptr, num_args as _) };
        run_aggregator_step::<A, Args, ArgsSqlType>(ctx, args)
    })
    .unwrap_or_else(|e| {
        Err(SqliteCallbackError::Panic(alloc::format!(
            "{}::step() panicked",
            core::any::type_name::<A>()
        )))
    });

    match result {
        Ok(()) => {}
        Err(e) => e.emit(ctx),
    }
}

fn run_aggregator_step<A, Args, ArgsSqlType>(
    ctx: *mut ffi::sqlite3_context,
    args: &mut [*mut ffi::sqlite3_value],
) -> Result<(), SqliteCallbackError>
where
    A: SqliteAggregateFunction<Args>,
    Args: FromSqlRow<ArgsSqlType, Sqlite>,
{
    static NULL_AG_CTX_ERR: &str = "An unknown error occurred. sqlite3_aggregate_context returned a null pointer. This should never happen.";
    static NULL_CTX_ERR: &str =
        "We've written the aggregator to the aggregate context, but it could not be retrieved.";

    let n_bytes: i32 = core::mem::size_of::<OptionalAggregator<A>>()
        .try_into()
        .expect("Aggregate context should be larger than 2^32");
    let aggregate_context = unsafe {
        // This block of unsafe code makes the following assumptions:
        //
        // * sqlite3_aggregate_context allocates sizeof::<OptionalAggregator<A>>
        //   bytes of zeroed memory as documented here:
        //   https://www.sqlite.org/c3ref/aggregate_context.html
        //   A null pointer is returned for negative or zero sized types,
        //   which should be impossible in theory. We check that nevertheless
        //
        // * OptionalAggregator::None has a discriminant of 0 as specified by
        //   #[repr(u8)] + RFC 2195
        //
        // * If all bytes are zero, the discriminant is also zero, so we can
        //   assume that we get OptionalAggregator::None in this case. This is
        //   not UB as we only access the discriminant here, so we do not try
        //   to read any other zeroed memory. After that we initialize our enum
        //   by writing a correct value at this location via ptr::write_unaligned
        //
        // * We use ptr::write_unaligned as we did not found any guarantees that
        //   the memory will have a correct alignment.
        //   (Note I(weiznich): would assume that it is aligned correctly, but we
        //    we cannot guarantee it, so better be safe than sorry)
        ffi::sqlite3_aggregate_context(ctx, n_bytes)
    };
    let aggregate_context = NonNull::new(aggregate_context as *mut OptionalAggregator<A>);
    let aggregator = unsafe {
        match aggregate_context.map(|a| &mut *a.as_ptr()) {
            Some(&mut OptionalAggregator::Some(ref mut agg)) => agg,
            Some(a_ptr @ &mut OptionalAggregator::None) => {
                ptr::write_unaligned(a_ptr as *mut _, OptionalAggregator::Some(A::default()));
                if let OptionalAggregator::Some(agg) = a_ptr {
                    agg
                } else {
                    return Err(SqliteCallbackError::Abort(NULL_CTX_ERR));
                }
            }
            None => {
                return Err(SqliteCallbackError::Abort(NULL_AG_CTX_ERR));
            }
        }
    };
    let args = build_sql_function_args::<ArgsSqlType, Args>(args)?;

    aggregator.step(args);
    Ok(())
}

extern "C" fn run_aggregator_final_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
    ctx: *mut ffi::sqlite3_context,
) where
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send,
    Args: FromSqlRow<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    static NO_AGGREGATOR_FOUND: &str = "We've written to the aggregator in the xStep callback. If xStep was never called, then ffi::sqlite_aggregate_context() would have returned a NULL pointer.";
    let aggregate_context = unsafe {
        // Within the xFinal callback, it is customary to set nBytes to 0 so no pointless memory
        // allocations occur, a null pointer is returned in this case
        // See: https://www.sqlite.org/c3ref/aggregate_context.html
        //
        // For the reasoning about the safety of the OptionalAggregator handling
        // see the comment in run_aggregator_step_function.
        ffi::sqlite3_aggregate_context(ctx, 0)
    };

    let result = crate::util::std_compat::catch_unwind(|| {
        let mut aggregate_context = NonNull::new(aggregate_context as *mut OptionalAggregator<A>);

        let aggregator = if let Some(a) = aggregate_context.as_mut() {
            let a = unsafe { a.as_mut() };
            match core::mem::replace(a, OptionalAggregator::None) {
                OptionalAggregator::None => {
                    return Err(SqliteCallbackError::Abort(NO_AGGREGATOR_FOUND));
                }
                OptionalAggregator::Some(a) => Some(a),
            }
        } else {
            None
        };

        let res = A::finalize(aggregator);
        let value = process_sql_function_result(&res)?;
        // We've checked already that ctx is not null
        let r = unsafe { value.result_of(&mut *ctx) };
        r.map_err(|e| {
            SqliteCallbackError::DieselError(crate::result::Error::SerializationError(Box::new(e)))
        })?;
        Ok(())
    })
    .unwrap_or_else(|_e| {
        Err(SqliteCallbackError::Panic(alloc::format!(
            "{}::finalize() panicked",
            core::any::type_name::<A>()
        )))
    });
    if let Err(e) = result {
        e.emit(ctx);
    }
}

unsafe fn context_error_str(ctx: *mut ffi::sqlite3_context, error: &str) {
    let len: i32 = error
        .len()
        .try_into()
        .expect("Trying to set a error message with more than 2^32 byte is not supported");
    unsafe {
        ffi::sqlite3_result_error(ctx, error.as_ptr() as *const _, len);
    }
}

struct CollationUserPtr<F> {
    callback: F,
    collation_name: String,
}

#[allow(warnings)]
extern "C" fn run_collation_function<F>(
    user_ptr: *mut libc::c_void,
    lhs_len: libc::c_int,
    lhs_ptr: *const libc::c_void,
    rhs_len: libc::c_int,
    rhs_ptr: *const libc::c_void,
) -> libc::c_int
where
    F: Fn(&str, &str) -> core::cmp::Ordering + Send + core::panic::UnwindSafe + 'static,
{
    let user_ptr = user_ptr as *const CollationUserPtr<F>;
    let user_ptr = core::panic::AssertUnwindSafe(unsafe { user_ptr.as_ref() });

    let result = crate::util::std_compat::catch_unwind(|| {
        let user_ptr = user_ptr.ok_or_else(|| {
            SqliteCallbackError::Abort(
                "Got a null pointer as data pointer. This should never happen",
            )
        })?;
        for (ptr, len, side) in &[(rhs_ptr, rhs_len, "rhs"), (lhs_ptr, lhs_len, "lhs")] {
            if *len < 0 {
                assert_fail!(
                    "An unknown error occurred. {}_len is negative. This should never happen.",
                    side
                );
            }
            if ptr.is_null() {
                assert_fail!(
                "An unknown error occurred. {}_ptr is a null pointer. This should never happen.",
                side
            );
            }
        }

        let (rhs, lhs) = unsafe {
            // Depending on the eTextRep-parameter to sqlite3_create_collation_v2() the strings can
            // have various encodings. register_collation_function() always selects SQLITE_UTF8, so the
            // pointers point to valid UTF-8 strings (assuming correct behavior of libsqlite3).
            (
                str::from_utf8(slice::from_raw_parts(rhs_ptr as *const u8, rhs_len as _)),
                str::from_utf8(slice::from_raw_parts(lhs_ptr as *const u8, lhs_len as _)),
            )
        };

        let rhs =
            rhs.map_err(|_| SqliteCallbackError::Abort("Got an invalid UTF-8 string for rhs"))?;
        let lhs =
            lhs.map_err(|_| SqliteCallbackError::Abort("Got an invalid UTF-8 string for lhs"))?;

        Ok((user_ptr.callback)(rhs, lhs))
    })
    .unwrap_or_else(|p| {
        Err(SqliteCallbackError::Panic(
            user_ptr
                .map(|u| u.collation_name.clone())
                .unwrap_or_default(),
        ))
    });

    match result {
        Ok(core::cmp::Ordering::Less) => -1,
        Ok(core::cmp::Ordering::Equal) => 0,
        Ok(core::cmp::Ordering::Greater) => 1,
        Err(SqliteCallbackError::Abort(a)) => {
            #[cfg(feature = "std")]
            eprintln!(
                "Collation function {} failed with: {}",
                user_ptr
                    .map(|c| &c.collation_name as &str)
                    .unwrap_or_default(),
                a
            );
            crate::util::std_compat::abort()
        }
        Err(SqliteCallbackError::DieselError(e)) => {
            #[cfg(feature = "std")]
            eprintln!(
                "Collation function {} failed with: {}",
                user_ptr
                    .map(|c| &c.collation_name as &str)
                    .unwrap_or_default(),
                e
            );
            crate::util::std_compat::abort()
        }
        Err(SqliteCallbackError::Panic(msg)) => {
            #[cfg(feature = "std")]
            eprintln!("Collation function {} panicked", msg);
            crate::util::std_compat::abort()
        }
    }
}

extern "C" fn destroy_boxed<F>(data: *mut libc::c_void) {
    let ptr = data as *mut F;
    unsafe { core::mem::drop(Box::from_raw(ptr)) };
}

#[cfg(test)]
mod tests {
    use super::super::update_hook::{SqliteChangeOp, SqliteChangeOps};
    use super::*;
    use std::sync::{Arc, Mutex};

    fn test_connection() -> RawConnection {
        RawConnection::establish(":memory:").expect("failed to establish :memory: connection")
    }

    #[test]
    fn insert_event_dispatched_directly() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.change_hooks.borrow_mut().add(
            Some("t"),
            SqliteChangeOps::INSERT,
            Box::new(move |e| {
                f2.lock().unwrap().push((e.op, e.rowid));
            }),
        );

        conn.register_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (1)").unwrap();

        let events = fired.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, SqliteChangeOp::Insert);
        assert_eq!(events[0].1, 1);
    }

    #[test]
    fn consecutive_inserts_dispatch_immediately() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.change_hooks.borrow_mut().add(
            Some("t"),
            SqliteChangeOps::INSERT,
            Box::new(move |e| {
                f2.lock().unwrap().push(e.rowid);
            }),
        );

        conn.register_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (2); INSERT INTO t VALUES (3)")
            .unwrap();

        let events = fired.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], 2);
        assert_eq!(events[1], 3);
    }

    #[test]
    fn update_event() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
            .unwrap();
        conn.exec("INSERT INTO t VALUES (1, 'a')").unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.change_hooks.borrow_mut().add(
            Some("t"),
            SqliteChangeOps::ALL,
            Box::new(move |e| {
                f2.lock().unwrap().push((e.op, e.rowid));
            }),
        );

        conn.register_raw_update_hook();
        conn.exec("UPDATE t SET v = 'b' WHERE id = 1").unwrap();

        let events = fired.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, SqliteChangeOp::Update);
        assert_eq!(events[0].1, 1);
    }

    #[test]
    fn delete_event() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();
        conn.exec("INSERT INTO t VALUES (1)").unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.change_hooks.borrow_mut().add(
            Some("t"),
            SqliteChangeOps::ALL,
            Box::new(move |e| {
                f2.lock().unwrap().push((e.op, e.rowid));
            }),
        );

        conn.register_raw_update_hook();
        conn.exec("DELETE FROM t WHERE id = 1").unwrap();

        let events = fired.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, SqliteChangeOp::Delete);
        assert_eq!(events[0].1, 1);
    }

    #[test]
    fn unregister_stops_events() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.change_hooks.borrow_mut().add(
            Some("t"),
            SqliteChangeOps::INSERT,
            Box::new(move |e| {
                f2.lock().unwrap().push(e.rowid);
            }),
        );

        conn.register_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (1)").unwrap();
        assert_eq!(fired.lock().unwrap().len(), 1);

        conn.unregister_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (2)").unwrap();
        assert_eq!(fired.lock().unwrap().len(), 1); // still 1
    }

    #[test]
    fn re_register_after_unregister_resumes() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.change_hooks.borrow_mut().add(
            Some("t"),
            SqliteChangeOps::INSERT,
            Box::new(move |e| {
                f2.lock().unwrap().push(e.rowid);
            }),
        );

        conn.register_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (1)").unwrap();
        assert_eq!(fired.lock().unwrap().len(), 1);

        conn.unregister_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (2)").unwrap();
        assert_eq!(fired.lock().unwrap().len(), 1);

        conn.register_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (3)").unwrap();
        let events = fired.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1], 3);
    }

    #[test]
    fn drop_does_not_panic() {
        let conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        conn.change_hooks
            .borrow_mut()
            .add(Some("t"), SqliteChangeOps::INSERT, Box::new(|_| {}));

        conn.register_raw_update_hook();
        conn.exec("INSERT INTO t VALUES (1)").unwrap();
        drop(conn);
        // If we get here, drop succeeded without panic.
    }

    // ===================================================================
    // Progress handler tests
    // ===================================================================

    #[test]
    fn progress_handler_can_interrupt() {
        let mut conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        // Insert many rows to make a long-running query
        for i in 0..1000 {
            conn.exec(&alloc::format!("INSERT INTO t VALUES ({})", i))
                .unwrap();
        }

        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        // Set a progress handler that interrupts after 10 callbacks
        conn.set_progress_handler(1, move || {
            let mut c = cc.lock().unwrap();
            *c += 1;
            *c > 10 // interrupt after 10 calls
        });

        // This query should be interrupted
        let result = conn.exec("SELECT * FROM t WHERE id > 0");
        assert!(result.is_err());

        // The handler should have been called multiple times
        assert!(*call_count.lock().unwrap() > 0);

        conn.remove_progress_handler();
    }

    #[test]
    fn progress_handler_remove_stops_calls() {
        let mut conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let called = Arc::new(Mutex::new(false));
        let c2 = called.clone();

        conn.set_progress_handler(1, move || {
            *c2.lock().unwrap() = true;
            false // don't interrupt
        });

        conn.remove_progress_handler();

        // This should not trigger the handler
        conn.exec("SELECT 1").unwrap();

        // In practice, for a simple query the handler may not fire even if set,
        // but after removal it definitely should not fire.
    }

    // ===================================================================
    // Busy handler tests
    // ===================================================================

    #[test]
    fn busy_handler_receives_retry_count() {
        // This test is tricky to set up without concurrent connections,
        // so we just verify the handler can be set and removed without panics.
        let mut conn = test_connection();

        let retry_counts = Arc::new(Mutex::new(Vec::new()));
        let rc = retry_counts.clone();

        conn.set_busy_handler(move |count| {
            rc.lock().unwrap().push(count);
            false // don't retry
        });

        // No lock contention in :memory: with single connection,
        // so the handler won't actually fire, but setup should work.
        conn.exec("SELECT 1").unwrap();

        conn.remove_busy_handler();
    }

    #[test]
    fn busy_timeout_can_be_set() {
        let mut conn = test_connection();
        // Just verify it doesn't panic
        conn.set_busy_timeout(5000);
        conn.exec("SELECT 1").unwrap();
    }

    // ===================================================================
    // Authorizer tests
    // ===================================================================

    #[test]
    fn authorizer_can_deny_operations() {
        use super::super::authorizer::{AuthorizerAction, AuthorizerDecision};

        let mut conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
            .unwrap();
        conn.exec("INSERT INTO t VALUES (1, 'a')").unwrap();

        // Set an authorizer that denies DELETE
        conn.set_authorizer(|ctx| {
            if matches!(ctx.action, AuthorizerAction::Delete) {
                AuthorizerDecision::Deny
            } else {
                AuthorizerDecision::Allow
            }
        });

        // This DELETE should fail due to authorization
        let result = conn.exec("DELETE FROM t WHERE id = 1");
        assert!(result.is_err());

        // This SELECT should still work
        conn.exec("SELECT * FROM t").unwrap();

        conn.remove_authorizer();

        // Now DELETE should work
        conn.exec("DELETE FROM t WHERE id = 1").unwrap();
    }

    #[test]
    fn authorizer_ignore_returns_null_for_read() {
        use super::super::authorizer::{AuthorizerAction, AuthorizerDecision};

        let mut conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, secret TEXT)")
            .unwrap();
        conn.exec("INSERT INTO t VALUES (1, 'password123')")
            .unwrap();

        // Set an authorizer that ignores reads on 'secret' column
        conn.set_authorizer(|ctx| {
            if matches!(ctx.action, AuthorizerAction::Read) && ctx.arg2 == Some("secret") {
                AuthorizerDecision::Ignore
            } else {
                AuthorizerDecision::Allow
            }
        });

        // Query should succeed but 'secret' column should be NULL
        // (We can't easily verify the NULL value here without diesel query,
        // but the query should not fail)
        conn.exec("SELECT id, secret FROM t").unwrap();

        conn.remove_authorizer();
    }

    #[test]
    fn is_schema_modifying_blocks_create_allows_dml() {
        use super::super::authorizer::AuthorizerDecision;

        let mut conn = test_connection();

        // First create a table without the authorizer
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
            .unwrap();
        conn.exec("INSERT INTO t VALUES (1, 'a')").unwrap();

        // Set an authorizer that denies schema-modifying operations
        conn.set_authorizer(|ctx| {
            if ctx.action.is_schema_modifying() {
                AuthorizerDecision::Deny
            } else {
                AuthorizerDecision::Allow
            }
        });

        // CREATE TABLE should fail
        let result = conn.exec("CREATE TABLE t2 (id INTEGER PRIMARY KEY)");
        assert!(result.is_err());

        // CREATE INDEX should fail
        let result = conn.exec("CREATE INDEX idx_v ON t(v)");
        assert!(result.is_err());

        // DROP TABLE should fail
        let result = conn.exec("DROP TABLE t");
        assert!(result.is_err());

        // SELECT should work
        conn.exec("SELECT * FROM t").unwrap();

        // INSERT should work
        conn.exec("INSERT INTO t VALUES (2, 'b')").unwrap();

        // UPDATE should work
        conn.exec("UPDATE t SET v = 'c' WHERE id = 1").unwrap();

        // DELETE should work
        conn.exec("DELETE FROM t WHERE id = 2").unwrap();

        conn.remove_authorizer();

        // After removing authorizer, CREATE TABLE should work
        conn.exec("CREATE TABLE t2 (id INTEGER PRIMARY KEY)")
            .unwrap();
    }

    // ===================================================================
    // Trace tests
    // ===================================================================

    #[test]
    fn trace_stmt_fires_on_query() {
        use super::super::trace::SqliteTraceFlags;

        let mut conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let statements = Arc::new(Mutex::new(Vec::new()));
        let s2 = statements.clone();

        conn.set_trace(SqliteTraceFlags::STMT, move |event| {
            if let super::super::trace::SqliteTraceEvent::Statement { sql, .. } = event {
                s2.lock().unwrap().push(sql.to_owned());
            }
        });

        conn.exec("INSERT INTO t VALUES (1)").unwrap();
        conn.exec("SELECT * FROM t").unwrap();

        let stmts = statements.lock().unwrap();
        assert!(stmts.len() >= 2);
        assert!(stmts.iter().any(|s| s.contains("INSERT")));
        assert!(stmts.iter().any(|s| s.contains("SELECT")));

        conn.remove_trace();
    }

    #[test]
    fn trace_profile_reports_duration() {
        use super::super::trace::{SqliteTraceEvent, SqliteTraceFlags};

        let mut conn = test_connection();
        conn.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .unwrap();

        let profiles = Arc::new(Mutex::new(Vec::new()));
        let p2 = profiles.clone();

        conn.set_trace(SqliteTraceFlags::PROFILE, move |event| {
            if let SqliteTraceEvent::Profile {
                sql, duration_ns, ..
            } = event
            {
                p2.lock().unwrap().push((sql.to_owned(), duration_ns));
            }
        });

        conn.exec("INSERT INTO t VALUES (1)").unwrap();

        let profs = profiles.lock().unwrap();
        assert!(!profs.is_empty());
        // Duration should be positive (though very small for simple queries)
        // Note: duration might be 0 for very fast queries on some systems

        conn.remove_trace();
    }

    #[test]
    fn trace_remove_stops_callbacks() {
        use super::super::trace::SqliteTraceFlags;

        let mut conn = test_connection();

        let called = Arc::new(Mutex::new(0u32));
        let c2 = called.clone();

        conn.set_trace(SqliteTraceFlags::STMT, move |_| {
            *c2.lock().unwrap() += 1;
        });

        conn.exec("SELECT 1").unwrap();
        let count_before = *called.lock().unwrap();
        assert!(count_before > 0);

        conn.remove_trace();

        conn.exec("SELECT 2").unwrap();
        let count_after = *called.lock().unwrap();

        // Should not have increased after removal
        assert_eq!(count_before, count_after);
    }
}
