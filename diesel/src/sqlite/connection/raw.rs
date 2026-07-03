#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;
use super::authorizer::{AuthorizerContext, AuthorizerDecision};
use super::functions::{build_sql_function_args, process_sql_function_result};
use super::limits::SqliteLimit;
use super::serialized_database::SerializedDatabase;
use super::stmt::ensure_sqlite_ok;
use super::trace::{SqliteTraceEvent, SqliteTraceFlags, TRACE_PROFILE, TRACE_ROW, TRACE_STMT};
use super::{BusyDecision, CommitDecision, ProgressDecision};
use super::{Sqlite, SqliteAggregateFunction};
use crate::deserialize::FromSqlRow;
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::HasSqlType;
use crate::sqlite::SqliteFunctionBehavior;
use alloc::borrow::{Cow, ToOwned};
use alloc::boxed::Box;
use alloc::ffi::{CString, NulError};
use alloc::string::{String, ToString};
use core::ffi as libc;
use core::ffi::CStr;
use core::num::NonZeroU32;
use core::ptr::NonNull;
use core::{mem, ptr, slice, str};

// `sqlite3_db_config()` option codes controlling whether ATTACH may create new
// database files (ATTACH_CREATE) or open them in write mode (ATTACH_WRITE).
// Introduced in SQLite 3.49.0 / `libsqlite3-sys` 0.35.0, but Diesel supports
// `libsqlite3-sys` >= 0.17.2, so we define them here to build against any
// supported version. On an older linked SQLite the `sqlite3_db_config()` call
// fails at runtime, which callers already handle.
pub(super) const SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE: i32 = 1020;
pub(super) const SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE: i32 = 1021;

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
    /// Boxed closure kept alive while the commit hook is registered.
    commit_hook: Option<Box<dyn FnMut() -> CommitDecision + Send>>,
    /// Boxed closure kept alive while the rollback hook is registered.
    rollback_hook: Option<Box<dyn FnMut() + Send>>,
    /// Boxed closure kept alive while the progress handler is registered.
    progress_hook: Option<Box<dyn FnMut() -> ProgressDecision + Send>>,
    /// Boxed closure kept alive while the WAL hook is registered.
    wal_hook: Option<Box<dyn Fn(&mut SqliteConnection, &str, u32) + Send>>,
    /// Boxed closure kept alive while the busy handler is registered.
    busy_handler: Option<Box<dyn FnMut(i32) -> BusyDecision + Send>>,
    /// Boxed closure kept alive while the authorizer is registered.
    authorizer_hook: Option<Box<dyn FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send>>,
    /// Boxed closure kept alive while the trace callback is registered.
    trace_hook: Option<Box<dyn FnMut(SqliteTraceEvent<'_>) + Send>>,
}

impl RawConnection {
    /// Wraps a borrowed `sqlite3` pointer this `RawConnection` does not own
    /// (kept in a `ManuallyDrop` for SQL-function callbacks, so `Drop` never runs).
    pub(super) fn from_ptr(conn: NonNull<ffi::sqlite3>) -> Self {
        RawConnection {
            internal_connection: conn,
            commit_hook: None,
            rollback_hook: None,
            progress_hook: None,
            wal_hook: None,
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
                Ok(RawConnection {
                    internal_connection: conn_pointer,
                    commit_hook: None,
                    rollback_hook: None,
                    progress_hook: None,
                    wal_hook: None,
                    busy_handler: None,
                    authorizer_hook: None,
                    trace_hook: None,
                })
            }
            err_code => {
                let message = super::error_message(err_code);
                // sqlite3_open_v2() may allocate a database connection handle
                // even on failure. To avoid a resource leak, it must be released
                // with sqlite3_close(). Passing a null pointer to sqlite3_close()
                // is a harmless no-op, so no null check is needed.
                // See: https://www.sqlite.org/c3ref/open.html
                unsafe { ffi::sqlite3_close(conn_pointer) };
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

    pub(super) fn last_insert_rowid(&self) -> i64 {
        unsafe { ffi::sqlite3_last_insert_rowid(self.internal_connection.as_ptr()) }
    }

    pub(super) fn register_sql_function<F, Ret, RetSqlType>(
        &self,
        fn_name: &str,
        num_args: usize,
        behavior: SqliteFunctionBehavior,
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
        let c_fn_name = Self::get_fn_name(fn_name)?;
        let flags = behavior.to_flags();
        let num_args = num_args
            .try_into()
            .map_err(|e| Error::SerializationError(Box::new(e)))?;
        // only create the pointer as last step here
        // as we can otherwise leak memory
        let callback_fn = Box::into_raw(Box::new(CustomFunctionUserPtr {
            callback: f,
            function_name: fn_name.to_owned(),
        }));

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                c_fn_name.as_ptr(),
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
        behavior: SqliteFunctionBehavior,
    ) -> QueryResult<()>
    where
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
        Args: FromSqlRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        let fn_name = Self::get_fn_name(fn_name)?;
        let flags = behavior.to_flags();
        let num_args = num_args
            .try_into()
            .map_err(|e| Error::SerializationError(Box::new(e)))?;

        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.internal_connection.as_ptr(),
                fn_name.as_ptr(),
                num_args,
                flags,
                core::ptr::null_mut(),
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
        let c_collation_name = Self::get_fn_name(collation_name)?;
        // only create the pointer as last step here as we otherwise could leak memory
        let callback_fn = Box::into_raw(Box::new(CollationUserPtr {
            callback: collation,
            collation_name: collation_name.to_owned(),
        }));

        let result = unsafe {
            ffi::sqlite3_create_collation_v2(
                self.internal_connection.as_ptr(),
                c_collation_name.as_ptr(),
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

    // SAFETY:
    // Any caller must ensure that the provided data buffer is valid and not modified until the database connection is closed
    // Sqlite's documentation states:
    // Applications must not modify the buffer P or invalidate it before the database connection D is closed.
    pub(super) unsafe fn deserialize(&mut self, data: &[u8]) -> QueryResult<()> {
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

    pub(super) fn set_limit(&self, limit: SqliteLimit, value: i32) -> i32 {
        unsafe { ffi::sqlite3_limit(self.internal_connection.as_ptr(), limit.to_ffi(), value) }
    }

    pub(super) fn get_limit(&self, limit: SqliteLimit) -> i32 {
        unsafe {
            // Passing -1 queries the current value without changing it
            ffi::sqlite3_limit(self.internal_connection.as_ptr(), limit.to_ffi(), -1)
        }
    }

    /// Set a boolean db_config option.
    pub(super) fn set_db_config_bool(&self, op: i32, value: bool) -> QueryResult<()> {
        let mut result_value: libc::c_int = 0;
        let new_value: libc::c_int = if value { 1 } else { 0 };

        let result = unsafe {
            ffi::sqlite3_db_config(
                self.internal_connection.as_ptr(),
                op,
                new_value,
                &mut result_value as *mut libc::c_int,
            )
        };

        ensure_sqlite_ok(result, self.internal_connection.as_ptr())
    }

    /// Get a boolean db_config option.
    pub(super) fn get_db_config_bool(&self, op: i32) -> QueryResult<bool> {
        let mut current_value: libc::c_int = 0;

        let result = unsafe {
            ffi::sqlite3_db_config(
                self.internal_connection.as_ptr(),
                op,
                -1_i32, // -1 queries without changing
                &mut current_value as *mut libc::c_int,
            )
        };

        ensure_sqlite_ok(result, self.internal_connection.as_ptr())?;
        Ok(current_value != 0)
    }

    fn get_fn_name(fn_name: &str) -> Result<CString, NulError> {
        CString::new(fn_name)
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

    pub(super) fn blob_open<'conn>(
        &'conn self,
        database_name: &str,
        table_name: &str,
        column_name: &str,
        row_id: i64,
    ) -> Result<super::sqlite_blob::SqliteReadOnlyBlob<'conn>, Error> {
        let database_name = alloc::ffi::CString::new(database_name)?;
        let column_name = alloc::ffi::CString::new(column_name)?;
        let table_name = alloc::ffi::CString::new(table_name)?;

        let mut blob: *mut ffi::sqlite3_blob = core::ptr::null_mut();

        // SAFETY: All variables are properly initialized
        let ret = unsafe {
            ffi::sqlite3_blob_open(
                self.internal_connection.as_ptr(),
                database_name.as_c_str().as_ptr(),
                table_name.as_c_str().as_ptr(),
                column_name.as_c_str().as_ptr(),
                row_id,
                0,
                &mut blob,
            )
        };

        Self::process_sql_function_result(ret)?;

        // SAFETY: `sqlite3_blob_open` initializes the `blob` variable IF the return value:
        //
        // > On success, SQLITE_OK is returned and the new BLOB handle is stored in *ppBlob.
        // > Otherwise an error code is returned and, unless the error code is SQLITE_MISUSE,
        // > *ppBlob is set to NULL.
        //
        // And we checked the `ret` value above
        let blob = unsafe { core::ptr::NonNull::new_unchecked(blob) };

        // SAFETY: According to the SQLite docs, this can only fail if an invalid pointer is passed
        let blob_size = unsafe { ffi::sqlite3_blob_bytes(blob.as_ptr()) };
        let blob_size = usize::try_from(blob_size).map_err(Error::IntegerConversion)?;

        Ok(super::sqlite_blob::SqliteReadOnlyBlob {
            blob,
            read_index: 0,
            blob_size,
            _pd: core::marker::PhantomData,
        })
    }

    /// Sets the commit hook, replacing any previous one.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw mut *boxed` and points to the heap
    /// allocation of the closure. The `commit_hook_trampoline` function
    /// matches the signature expected by `sqlite3_commit_hook`. The pointer
    /// remains valid because we store `boxed` in `self.commit_hook` below,
    /// keeping it alive for the lifetime of this `RawConnection`.
    pub(super) fn set_commit_hook<F>(&mut self, hook: F)
    where
        F: FnMut() -> CommitDecision + Send + 'static,
    {
        let mut boxed: Box<dyn FnMut() -> CommitDecision + Send> = Box::new(hook);
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
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing `None` as the hook and `null_mut()` as the user
    /// data unregisters any existing commit hook. The hook is unregistered
    /// before dropping `self.commit_hook` to prevent callbacks from firing
    /// during cleanup.
    pub(super) fn remove_commit_hook(&mut self) {
        unsafe {
            ffi::sqlite3_commit_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.commit_hook = None;
    }

    /// Sets the rollback hook, replacing any previous one.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw mut *boxed` and points to the heap
    /// allocation of the closure. The `rollback_hook_trampoline` function
    /// matches the signature expected by `sqlite3_rollback_hook`. The pointer
    /// remains valid because we store `boxed` in `self.rollback_hook` below,
    /// keeping it alive for the lifetime of this `RawConnection`.
    pub(super) fn set_rollback_hook<F>(&mut self, hook: F)
    where
        F: FnMut() + Send + 'static,
    {
        let mut boxed: Box<dyn FnMut() + Send> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;

        unsafe {
            ffi::sqlite3_rollback_hook(
                self.internal_connection.as_ptr(),
                Some(rollback_hook_trampoline::<F>),
                ptr,
            );
        }

        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.rollback_hook = Some(boxed);
    }

    /// Removes the rollback hook.
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing `None` as the hook and `null_mut()` as the user
    /// data unregisters any existing rollback hook. The hook is unregistered
    /// before dropping `self.rollback_hook` to prevent callbacks from firing
    /// during cleanup.
    pub(super) fn remove_rollback_hook(&mut self) {
        unsafe {
            ffi::sqlite3_rollback_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.rollback_hook = None;
    }

    /// Sets the progress handler, replacing any previous one.
    ///
    /// `n` is the approximate number of VM instructions between callbacks.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw mut *boxed` and points to the heap
    /// allocation of the closure. The `progress_handler_trampoline` function
    /// matches the signature expected by `sqlite3_progress_handler`. The
    /// pointer remains valid because we store `boxed` in `self.progress_hook`
    /// below, keeping it alive for the lifetime of this `RawConnection`.
    pub(super) fn set_progress_handler<F>(&mut self, n: NonZeroU32, hook: F)
    where
        F: FnMut() -> ProgressDecision + Send + 'static,
    {
        let mut boxed: Box<dyn FnMut() -> ProgressDecision + Send> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;

        // `sqlite3_progress_handler` takes a c_int. A value above `i32::MAX`
        // would wrap to a non-positive number and disable the handler, so clamp
        // it to `i32::MAX` instead.
        let n = i32::try_from(n.get()).unwrap_or(i32::MAX);

        unsafe {
            ffi::sqlite3_progress_handler(
                self.internal_connection.as_ptr(),
                n,
                Some(progress_handler_trampoline::<F>),
                ptr,
            );
        }

        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.progress_hook = Some(boxed);
    }

    /// Removes the progress handler.
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing `None` as the handler and `null_mut()` as the user
    /// data unregisters any existing progress handler. The handler is
    /// unregistered before dropping `self.progress_hook` to prevent callbacks
    /// from firing during cleanup.
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

    /// Sets the WAL hook, replacing any previous one.
    ///
    /// The callback receives a borrowed `&mut SqliteConnection`, the database
    /// name (e.g. `"main"`) and the number of pages currently in the WAL file.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw const *boxed` and points to the heap
    /// allocation of the closure. The `wal_hook_trampoline` function matches the
    /// signature expected by `sqlite3_wal_hook`. The pointer remains valid
    /// because we store `boxed` in `self.wal_hook` below, keeping it alive for
    /// the lifetime of this `RawConnection`.
    pub(super) fn set_wal_hook<F>(&mut self, hook: F)
    where
        F: Fn(&mut SqliteConnection, &str, u32) + Send + 'static,
    {
        let boxed: Box<dyn Fn(&mut SqliteConnection, &str, u32) + Send> = Box::new(hook);
        let ptr = &raw const *boxed as *mut libc::c_void;

        unsafe {
            ffi::sqlite3_wal_hook(
                self.internal_connection.as_ptr(),
                Some(wal_hook_trampoline::<F>),
                ptr,
            );
        }

        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.wal_hook = Some(boxed);
    }

    /// Removes the WAL hook.
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing `None` as the hook and `null_mut()` as the user
    /// data unregisters any existing WAL hook. The hook is unregistered before
    /// dropping `self.wal_hook` to prevent callbacks from firing during
    /// cleanup.
    pub(super) fn remove_wal_hook(&mut self) {
        unsafe {
            ffi::sqlite3_wal_hook(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.wal_hook = None;
    }

    /// Sets the busy handler, replacing any previous one.
    ///
    /// Only one busy handler can be active at a time. Setting this clears any
    /// busy timeout previously set with `set_busy_timeout`.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw mut *boxed` and points to the heap
    /// allocation of the closure. The `busy_handler_trampoline` function
    /// matches the signature expected by `sqlite3_busy_handler`. The pointer
    /// remains valid because we store `boxed` in `self.busy_handler` below,
    /// keeping it alive for the lifetime of this `RawConnection`.
    pub(super) fn set_busy_handler<F>(&mut self, hook: F)
    where
        F: FnMut(i32) -> BusyDecision + Send + 'static,
    {
        let mut boxed: Box<dyn FnMut(i32) -> BusyDecision + Send> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;

        unsafe {
            ffi::sqlite3_busy_handler(
                self.internal_connection.as_ptr(),
                Some(busy_handler_trampoline::<F>),
                ptr,
            );
        }

        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.busy_handler = Some(boxed);
    }

    /// Removes the busy handler.
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing `None` as the hook and `null_mut()` as the user
    /// data unregisters any existing busy handler. The hook is unregistered
    /// before dropping `self.busy_handler` to prevent callbacks from firing
    /// during cleanup.
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
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. `sqlite3_busy_timeout` installs its own internal busy
    /// handler, so the stored `busy_handler` is dropped afterwards to release
    /// the now-unused closure.
    pub(super) fn set_busy_timeout(&mut self, ms: i32) {
        unsafe {
            ffi::sqlite3_busy_timeout(self.internal_connection.as_ptr(), ms);
        }
        self.busy_handler = None;
    }

    /// Sets the authorizer callback, replacing any previous one. Only one
    /// can be active at a time per connection.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw mut *boxed` and points to the heap
    /// allocation of the closure. The `authorizer_trampoline` function
    /// matches the signature expected by `sqlite3_set_authorizer`. The pointer
    /// remains valid because we store `boxed` in `self.authorizer_hook` below,
    /// keeping it alive for the lifetime of this `RawConnection`.
    pub(super) fn set_authorizer<F>(&mut self, hook: F)
    where
        F: FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send + 'static,
    {
        let mut boxed: Box<dyn FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send> =
            Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;

        unsafe {
            ffi::sqlite3_set_authorizer(
                self.internal_connection.as_ptr(),
                Some(authorizer_trampoline::<F>),
                ptr,
            );
        }

        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.authorizer_hook = Some(boxed);
    }

    /// Removes the authorizer callback.
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing `None` as the callback and `null_mut()` as the
    /// user data unregisters any existing authorizer. The authorizer is
    /// unregistered before dropping `self.authorizer_hook` to prevent
    /// callbacks from firing during cleanup.
    pub(super) fn remove_authorizer(&mut self) {
        unsafe {
            ffi::sqlite3_set_authorizer(self.internal_connection.as_ptr(), None, ptr::null_mut());
        }
        self.authorizer_hook = None;
    }

    /// Sets a trace callback, replacing any previous one.
    ///
    /// The callback is invoked for SQL execution tracing based on the
    /// provided event mask.
    ///
    /// # Safety
    ///
    /// The `ptr` is derived from `&raw mut *boxed` and points to the heap
    /// allocation of the closure. The `trace_trampoline` function matches the
    /// signature expected by `sqlite3_trace_v2`. The pointer remains valid
    /// because we store `boxed` in `self.trace_hook` below, keeping it alive
    /// for the lifetime of this `RawConnection`.
    pub(super) fn set_trace<F>(&mut self, mask: SqliteTraceFlags, hook: F)
    where
        F: FnMut(SqliteTraceEvent<'_>) + Send + 'static,
    {
        let mut boxed: Box<dyn FnMut(SqliteTraceEvent<'_>) + Send> = Box::new(hook);
        let ptr = &raw mut *boxed as *mut libc::c_void;

        unsafe {
            ffi::sqlite3_trace_v2(
                self.internal_connection.as_ptr(),
                mask.bits(),
                Some(trace_trampoline::<F>),
                ptr,
            );
        }

        // The old Box (if any) is dropped here after SQLite has already
        // switched to the new callback, preventing use-after-free.
        self.trace_hook = Some(boxed);
    }

    /// Removes the trace callback.
    ///
    /// # Safety
    ///
    /// `self.internal_connection` is a valid pointer to an open SQLite
    /// connection. Passing a mask of `0`, `None` as the callback, and
    /// `null_mut()` as the user data unregisters any existing trace callback.
    /// The callback is unregistered before dropping `self.trace_hook` to
    /// prevent callbacks from firing during cleanup.
    pub(super) fn remove_trace(&mut self) {
        unsafe {
            ffi::sqlite3_trace_v2(self.internal_connection.as_ptr(), 0, None, ptr::null_mut());
        }
        self.trace_hook = None;
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        use crate::util::std_compat::panicking;

        // Unregister before close so the boxed closures drop before sqlite3_close.
        self.remove_commit_hook();
        self.remove_rollback_hook();
        self.remove_progress_handler();
        self.remove_wal_hook();
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
    // conn holds a `Box<dyn FnMut() -> CommitDecision + Send>` hook field which is not UnwindSafe.
    // The ManuallyDrop wrapper ensures we never run RawConnection's Drop.
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
    let aggregator = unsafe {
        const {
            if core::mem::size_of::<*mut A>() == 0 {
                panic!(
                    "The pointer size is zero, that's unexpected.\
                        If you ever see this error message open a issue\
                        describing your environment"
                );
            }
        }
        // sqlite3_aggregate_context will return a memory allocation of the requested
        // size. For the first call this will be zeroed, for any future call in the same execution
        // this will contain the value we wrote into it.
        //
        // We write just a pointer to rust allocated memory in there to
        // have the rust side deal with layout and alignment of our aggregator
        let ctx = ffi::sqlite3_aggregate_context(
            ctx,
            core::mem::size_of::<*mut A>()
                .try_into()
                .expect("Memory size of a pointer is smaller than i32::MAX"),
        )
        // we cast the returned memory here to be a pointer to the aggregate instance
        .cast::<*mut A>();
        // we are interested in the inner pointer
        let inner = &mut *ctx;
        // if the inner pointer is null we the aggregate_step
        // function is executed the first time and we need to create the actual
        // aggregator
        if inner.is_null() {
            // for that we allocate a box and turn it into a raw pointer
            // by leaking the memory
            let obj = Box::into_raw(Box::new(A::default()));
            *inner = obj;
        }
        // at this point the inner value is never null
        // as we initialised in in the null branch above,
        // therefore it's sound to dereference the pointer here
        &mut **inner
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
    let result = crate::util::std_compat::catch_unwind(|| {
        let aggregator = unsafe {
            // Get back the aggregated context
            // This might be null
            let ctx = ffi::sqlite3_aggregate_context(
                ctx,
                // use zero sized allocation here to not allocate if this is the first call to `sqlite3_aggregate_context`
                0,
            )
            // the allocation contains a pointer to the actual aggregator
            .cast::<*mut A>();
            // if the context was not allocated yet
            // we get back a null pointer here due to
            // the requested zero sized allocation
            if ctx.is_null() {
                None
            } else {
                // from this point we are interested in the inner pointer
                // we checked above that this pointer is not null
                // so it's sound to dereference it
                let inner = &mut *ctx;
                if inner.is_null() {
                    // if the inner pointer is null the aggregator has not been initialized
                    None
                } else {
                    // if it's not null
                    // we need to construct back the box and move out the
                    // value to correctly deallocate the allocation
                    let value = Box::from_raw(*inner);
                    let value = Some(*value);
                    // we also want to write a null pointer back to the
                    // context to make sure that there is no dangling pointer left
                    *inner = core::ptr::null_mut();
                    value
                }
            }
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
    let len: i32 = error.len().try_into().unwrap_or(i32::MAX);
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

/// C trampoline for `sqlite3_commit_hook`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::commit_hook`.
unsafe extern "C" fn commit_hook_trampoline<F>(user_data: *mut libc::c_void) -> libc::c_int
where
    F: FnMut() -> CommitDecision,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::commit_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f()
    }));

    match result {
        Ok(CommitDecision::Rollback) => 1,
        Ok(CommitDecision::Proceed) => 0,
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

/// C trampoline for `sqlite3_progress_handler`.
///
/// # Safety
///
/// `user_data` must point to a live `F` stored in `RawConnection::progress_hook`.
unsafe extern "C" fn progress_handler_trampoline<F>(user_data: *mut libc::c_void) -> libc::c_int
where
    F: FnMut() -> ProgressDecision,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::progress_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f()
    }));

    match result {
        Ok(ProgressDecision::Interrupt) => 1,
        Ok(ProgressDecision::Continue) => 0,
        Err(_) => {
            assert_fail!("Panic in sqlite3_progress_handler trampoline. ");
        }
    }
}

/// C trampoline for `sqlite3_wal_hook`.
///
/// # Safety
///
/// `user_data` must point to a live `F` in `RawConnection::wal_hook`. `db` is
/// the `sqlite3` handle that fired the hook, open and unlocked for the duration
/// of the call.
unsafe extern "C" fn wal_hook_trampoline<F>(
    user_data: *mut libc::c_void,
    db: *mut ffi::sqlite3,
    db_name: *const libc::c_char,
    n_pages: libc::c_int,
) -> libc::c_int
where
    F: Fn(&mut SqliteConnection, &str, u32),
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::wal_hook`.
        let f = unsafe { &*(user_data as *const F) };

        // SAFETY: when non-null, `db_name` is a valid C string from SQLite. Use
        // a lossy conversion so a pathological name cannot abort, and map null
        // to "".
        let db_name: Cow<'_, str> = if db_name.is_null() {
            Cow::Borrowed("")
        } else {
            unsafe { CStr::from_ptr(db_name) }.to_string_lossy()
        };
        // SQLite always reports a non-negative page count. Clamp defensively.
        let n_pages = u32::try_from(n_pages).unwrap_or(0);

        let Some(db) = NonNull::new(db) else {
            return;
        };

        // SAFETY: per the `sqlite3_wal_hook` docs the commit is complete and the
        // write-lock released, so `db` may be used. `with_borrowed_connection`
        // finalizes statements on return and never closes `db`. The callback is
        // `Fn`, so a re-entrant call (a committing write inside it) is sound.
        unsafe {
            SqliteConnection::with_borrowed_connection(db, |conn| f(conn, &db_name, n_pages));
        }
    }));

    if result.is_err() {
        assert_fail!("Panic in sqlite3_wal_hook trampoline. ");
    }

    ffi::SQLITE_OK
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
    F: FnMut(i32) -> BusyDecision,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::busy_handler`.
        let f = unsafe { &mut *(user_data as *mut F) };
        f(retry_count)
    }));

    match result {
        Ok(BusyDecision::Retry) => 1,
        Ok(BusyDecision::GiveUp) => 0,
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
    // Convert a nullable C string argument to `Option<&str>`. A null pointer or
    // a non-UTF-8 string both map to `None`. The borrow is tied to this call via
    // the generic lifetime, and the resulting `&str` only lives inside the
    // `AuthorizerContext` passed to the callback, which cannot escape the call.
    fn to_str<'a>(ptr: *const libc::c_char) -> Option<&'a str> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: per the `sqlite3_set_authorizer` contract a non-null
            // pointer is a valid C string for the duration of the call.
            unsafe { CStr::from_ptr(ptr) }.to_str().ok()
        }
    }

    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::authorizer_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };

        let ctx = AuthorizerContext::from_ffi(
            action_code,
            to_str(arg1),
            to_str(arg2),
            to_str(db_name),
            to_str(accessor),
        );

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
    F: FnMut(SqliteTraceEvent<'_>) + Send + 'static,
{
    let result = crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
        // SAFETY: `user_data` points to a live `F` in `RawConnection::trace_hook`.
        let f = unsafe { &mut *(user_data as *mut F) };

        // The callback is invoked inside each arm so the lossy `Cow` holding the
        // SQL text outlives the `&str` borrow handed to it. `TRACE_STMT`,
        // `TRACE_PROFILE`, and `TRACE_ROW` are the `u32`-normalized event codes
        // from the `trace` module, so they match `event_code` directly.
        match event_code {
            TRACE_STMT => {
                // p = sqlite3_stmt*, x = const char* (unexpanded SQL).
                let stmt_ptr = p as *mut ffi::sqlite3_stmt;
                let sql_ptr = x as *const libc::c_char;
                if sql_ptr.is_null() {
                    return;
                }
                // SAFETY: a non-null `x` is a valid C string from SQLite. Use a
                // lossy conversion so non-UTF-8 SQL cannot abort the process.
                let sql = unsafe { CStr::from_ptr(sql_ptr) }.to_string_lossy();
                let readonly =
                    !stmt_ptr.is_null() && unsafe { ffi::sqlite3_stmt_readonly(stmt_ptr) != 0 };
                f(SqliteTraceEvent::Statement {
                    sql: &sql,
                    readonly,
                });
            }
            TRACE_PROFILE => {
                // p = sqlite3_stmt*, x = sqlite3_int64* (nanoseconds).
                let stmt_ptr = p as *mut ffi::sqlite3_stmt;
                let duration_ns = unsafe {
                    // x points to a sqlite3_int64. The duration is non-negative.
                    (x as *const ffi::sqlite3_int64).as_ref()
                }
                .copied()
                .unwrap_or_default()
                .cast_unsigned();

                let readonly =
                    !stmt_ptr.is_null() && unsafe { ffi::sqlite3_stmt_readonly(stmt_ptr) != 0 };
                let sql_ptr = if stmt_ptr.is_null() {
                    core::ptr::null()
                } else {
                    unsafe { ffi::sqlite3_sql(stmt_ptr) }
                };
                // SAFETY: when non-null, `sqlite3_sql` returns a valid C string.
                let sql = if sql_ptr.is_null() {
                    Cow::Borrowed("")
                } else {
                    unsafe { CStr::from_ptr(sql_ptr) }.to_string_lossy()
                };
                f(SqliteTraceEvent::Profile {
                    sql: &sql,
                    duration_ns,
                    readonly,
                });
            }
            TRACE_ROW => f(SqliteTraceEvent::Row),
            // Unknown or unhandled events are ignored. CLOSE is intentionally
            // not handled: diesel removes the trace before closing the
            // connection, so it never fires.
            _ => {}
        }
    }));

    if result.is_err() {
        assert_fail!("Panic in sqlite3_trace_v2 trampoline. ");
    }

    0 // Return value is currently unused by SQLite
}
