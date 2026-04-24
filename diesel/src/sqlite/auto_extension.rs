#![allow(unsafe_code)]

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use crate::result::Error::DatabaseError;
use crate::result::*;
use alloc::boxed::Box;
use alloc::string::ToString;

/// Entry point signature for SQLite auto-extensions.
///
/// This matches the [`sqlite3_loadext_entry`][typedef] typedef in
/// `sqlite3ext.h`:
///
/// ```c
/// int entryPoint(sqlite3 *db, char **pzErrMsg, const sqlite3_api_routines *pThunk);
/// ```
///
/// [typedef]: https://sqlite.org/src/file/src/sqlite3ext.h (search `sqlite3_loadext_entry`)
///
/// - `db`: The database connection being opened.
/// - `pz_err_msg`: If set, must be allocated via [`sqlite3_mprintf`][mprintf]
///   (not the Rust allocator). SQLite will call [`sqlite3_free`][free] on it
///   ([source][free_src]).
/// - `p_api`: Pointer to the SQLite API routines table. May be NULL on
///   builds compiled with `SQLITE_OMIT_LOAD_EXTENSION` ([source][null_src]).
///
/// [mprintf]: https://www.sqlite.org/c3ref/mprintf.html
/// [free]: https://www.sqlite.org/c3ref/free.html
/// [free_src]: https://sqlite.org/src/file/src/loadext.c (search `sqlite3_free(zErrmsg)` in `sqlite3AutoLoadExtensions`)
/// [null_src]: https://sqlite.org/src/file/src/loadext.c (search `SQLITE_OMIT_LOAD_EXTENSION` in `sqlite3AutoLoadExtensions`)
///
/// Return `SQLITE_OK` (0) on success. Do **not** return
/// `SQLITE_OK_LOAD_PERMANENTLY` (256) — see [`register_auto_extension`]
/// for details.
///
/// **Note on `sqlite-loadable-rs` interop:** that crate's generated entry
/// points use `*mut sqlite3_api_routines` and `c_uint` return type, which
/// differ from this signature (`*const` and `c_int`). You may need a thin
/// wrapper function to bridge the types.
pub type AutoExtensionEntryPoint = unsafe extern "C" fn(
    db: *mut ffi::sqlite3,
    pz_err_msg: *mut *mut core::ffi::c_char,
    p_api: *const ffi::sqlite3_api_routines,
) -> core::ffi::c_int;

/// Registers an auto-extension entry point that will run for **all** future
/// SQLite connections opened in this process, including non-Diesel ones.
///
/// This is a thin wrapper around [`sqlite3_auto_extension`][docs].
///
/// [docs]: https://www.sqlite.org/c3ref/auto_extension.html
///
/// # When to call
///
/// Call this **before** creating any connection pools or opening connections.
/// Auto-extensions run in [registration order][order]; the first extension
/// that returns a non-`SQLITE_OK` value aborts the remaining extensions for
/// that connection and causes the open to fail.
///
/// Registering the same function pointer more than once is a
/// [no-op][dedup] — SQLite deduplicates registrations.
///
/// [order]: https://sqlite.org/src/file/src/loadext.c (search `sqlite3AutoLoadExtensions` — loop iterates `aExt` from index 0; first non-zero `rc` sets `go = 0`)
/// [dedup]: https://www.sqlite.org/c3ref/auto_extension.html
///
/// # Example
///
/// ```rust
/// #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
/// extern crate libsqlite3_sys as ffi;
/// #[cfg(all(target_family = "wasm", target_os = "unknown"))]
/// extern crate sqlite_wasm_rs as ffi;
///
/// use diesel::prelude::*;
/// use diesel::sqlite::{register_auto_extension, reset_auto_extension, SqliteConnection};
///
/// // An auto-extension entry point — typically provided by a C library
/// // linked into your binary (e.g., Spatialite, sqlite-vec) or written
/// // using `sqlite-loadable-rs`.
/// unsafe extern "C" fn my_ext_init(
///     _db: *mut ffi::sqlite3,
///     _pz_err_msg: *mut *mut core::ffi::c_char,
///     _p_api: *const ffi::sqlite3_api_routines,
/// ) -> core::ffi::c_int {
///     0 // SQLITE_OK
/// }
///
/// // Register before opening any connections.
/// unsafe { register_auto_extension(my_ext_init).unwrap() };
///
/// // All future connections get the extension automatically.
/// let mut conn = SqliteConnection::establish(":memory:").unwrap();
/// # reset_auto_extension();
/// ```
///
/// # Warning: `SQLITE_OK_LOAD_PERMANENTLY`
///
/// Do **not** return `SQLITE_OK_LOAD_PERMANENTLY` (256) from an
/// auto-extension callback. SQLite's [`sqlite3AutoLoadExtensions`][autoload]
/// treats any non-zero return as an error and stops executing remaining
/// extensions. However, because Diesel opens connections without
/// `SQLITE_OPEN_EXRESCODE`, the [default error mask][errmask] (`0xff`)
/// silently masks `0x100` to `0x00`, causing the connection to appear to
/// succeed while remaining extensions are skipped. Always return
/// `SQLITE_OK` (0) on success.
///
/// This is particularly relevant for [`sqlite-loadable-rs`][slrs] users:
/// `#[sqlite_entrypoint_permanent]` [returns 256][slrs256] and must **not**
/// be used with auto-extensions — use `#[sqlite_entrypoint]` instead.
///
/// [autoload]: https://sqlite.org/src/file/src/loadext.c (search `sqlite3AutoLoadExtensions` — `rc != 0` sets `go = 0`)
/// [errmask]: https://sqlite.org/src/file/src/main.c (search `errMask` in `openDatabase`)
/// [slrs]: https://github.com/asg017/sqlite-loadable-rs
/// [slrs256]: https://github.com/asg017/sqlite-loadable-rs/blob/main/src/entrypoints.rs (search `register_entrypoint_load_permanently` — returns hardcoded `256`)
///
/// # Safety
///
/// The caller must guarantee that `entry_point`:
///
/// - **Does not unwind.** Panics across the FFI boundary are undefined
///   behavior.
/// - **Does not close `db`** or open recursive connections on it.
/// - **Does not call** [`register_auto_extension`], [`cancel_auto_extension`],
///   or [`reset_auto_extension`]. Although the [mutex is released][mutex]
///   before the callback runs, mutating the extension list while SQLite is
///   iterating it may cause extensions to be skipped or invoked twice.
/// - **Is safe to call from multiple threads** simultaneously, since
///   concurrent connection opens may [invoke it in parallel][mutex].
/// - Only allocates `*pz_err_msg` with [`sqlite3_mprintf`][mprintf], never
///   the Rust allocator (SQLite will call [`sqlite3_free`][free] on it).
/// - Returns `SQLITE_OK` (0) on success.
/// - Is aware that `p_api` may be [NULL on `SQLITE_OMIT_LOAD_EXTENSION`
///   builds][null_src] (common in WASM). Extensions that dereference
///   `p_api` without a null check will crash on such builds.
///
/// [mutex]: https://sqlite.org/src/file/src/loadext.c (search `sqlite3AutoLoadExtensions` — `sqlite3_mutex_leave` before `xInit` call)
/// [mprintf]: https://www.sqlite.org/c3ref/mprintf.html
/// [free]: https://www.sqlite.org/c3ref/free.html
/// [null_src]: https://sqlite.org/src/file/src/loadext.c (search `SQLITE_OMIT_LOAD_EXTENSION` in `sqlite3AutoLoadExtensions`)
pub unsafe fn register_auto_extension(entry_point: AutoExtensionEntryPoint) -> QueryResult<()> {
    let result = unsafe { ffi::sqlite3_auto_extension(Some(entry_point)) };
    if result == ffi::SQLITE_OK {
        Ok(())
    } else {
        let error_message = ffi::code_to_str(result);
        Err(DatabaseError(
            DatabaseErrorKind::Unknown,
            Box::new(error_message.to_string()),
        ))
    }
}

/// Removes a previously registered auto-extension.
///
/// Returns `true` if the extension was found and removed, `false` if it was
/// not registered ([docs][cancel_docs]).
///
/// This function is safe because it only [compares and removes a
/// pointer][cancel_src] from SQLite's internal list — no callback is
/// invoked.
///
/// [cancel_docs]: https://www.sqlite.org/c3ref/cancel_auto_extension.html
/// [cancel_src]: https://sqlite.org/src/file/src/loadext.c (search `sqlite3_cancel_auto_extension`)
pub fn cancel_auto_extension(entry_point: AutoExtensionEntryPoint) -> bool {
    unsafe { ffi::sqlite3_cancel_auto_extension(Some(entry_point)) != 0 }
}

/// Clears **all** registered auto-extensions ([docs][reset_docs]).
///
/// After this call, no auto-extensions will run for newly opened connections.
///
/// [reset_docs]: https://www.sqlite.org/c3ref/reset_auto_extension.html
pub fn reset_auto_extension() {
    unsafe { ffi::sqlite3_reset_auto_extension() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::Integer;
    use crate::sqlite::SqliteConnection;

    // A simple auto-extension that registers a SQL function `test_auto_ext_42`
    // which returns the integer 42.
    unsafe extern "C" fn test_ext_init(
        db: *mut ffi::sqlite3,
        _pz_err_msg: *mut *mut core::ffi::c_char,
        _p_api: *const ffi::sqlite3_api_routines,
    ) -> core::ffi::c_int {
        unsafe extern "C" fn return_42(
            ctx: *mut ffi::sqlite3_context,
            _argc: core::ffi::c_int,
            _argv: *mut *mut ffi::sqlite3_value,
        ) {
            unsafe { ffi::sqlite3_result_int(ctx, 42) };
        }

        let name = c"test_auto_ext_42";
        unsafe {
            ffi::sqlite3_create_function_v2(
                db,
                name.as_ptr(),
                0,
                ffi::SQLITE_UTF8 | ffi::SQLITE_DETERMINISTIC,
                core::ptr::null_mut(),
                Some(return_42),
                None,
                None,
                None,
            )
        }
    }

    fn open_memory_connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").expect("Failed to open :memory: connection")
    }

    /// RAII guard that calls `reset_auto_extension()` on drop, ensuring
    /// global state is cleaned up even if a test panics.
    struct TestResetGuard;

    impl Drop for TestResetGuard {
        fn drop(&mut self) {
            reset_auto_extension();
        }
    }

    // All sub-tests are combined into a single #[test] function because
    // `sqlite3_auto_extension` is process-global state and cannot be
    // safely mutated from parallel test threads.
    #[test]
    fn auto_extension_lifecycle() {
        let _guard = TestResetGuard;
        reset_auto_extension();

        // -- 1. register + new connection has the function --
        unsafe { register_auto_extension(test_ext_init).unwrap() };

        let mut conn = open_memory_connection();
        let result: i32 = sql::<Integer>("SELECT test_auto_ext_42()")
            .get_result(&mut conn)
            .expect("auto-extension function should be available");
        assert_eq!(result, 42);

        // -- 2. cancel + new connection does NOT have the function --
        let removed = cancel_auto_extension(test_ext_init);
        assert!(
            removed,
            "cancel should return true for registered extension"
        );

        let mut conn = open_memory_connection();
        let result = crate::sql_query("SELECT test_auto_ext_42()").execute(&mut conn);
        assert!(
            result.is_err(),
            "function should not be available after cancel"
        );

        // -- 3. cancel returns false for unregistered --
        let removed = cancel_auto_extension(test_ext_init);
        assert!(
            !removed,
            "cancel should return false for unregistered extension"
        );

        // -- 4. reset clears all --
        unsafe { register_auto_extension(test_ext_init).unwrap() };
        reset_auto_extension();

        let mut conn = open_memory_connection();
        let result = crate::sql_query("SELECT test_auto_ext_42()").execute(&mut conn);
        assert!(
            result.is_err(),
            "function should not be available after reset"
        );

        // -- 5. duplicate registration is idempotent --
        unsafe {
            register_auto_extension(test_ext_init).unwrap();
            register_auto_extension(test_ext_init).unwrap();
        }

        let mut conn = open_memory_connection();
        let result: i32 = sql::<Integer>("SELECT test_auto_ext_42()")
            .get_result(&mut conn)
            .expect("function should be available after duplicate registration");
        assert_eq!(result, 42);

        // Note: a "failing extension causes connection open to fail" test is
        // intentionally omitted here. Registering a failing auto-extension
        // poisons ALL concurrent connection opens process-wide, which would
        // cause other parallel tests to fail.
        // _guard drops here, ensuring reset even on panic
    }
}
