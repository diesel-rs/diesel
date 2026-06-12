#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::SqliteConnection;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use core::ffi::{c_char, c_int};

/// The fn type `libsqlite3-sys` expects for `sqlite3_auto_extension`, used only
/// to name the private trampoline below.
type RawAutoExtension = unsafe extern "C" fn(
    db: *mut ffi::sqlite3,
    pz_err_msg: *mut *mut c_char,
    p_api: *const ffi::sqlite3_api_routines,
) -> c_int;

/// Registers an auto-extension that runs for every SQLite connection opened in
/// this process, including non-Diesel ones.
///
/// This is a safe wrapper around [`sqlite3_auto_extension`][docs]. The callback
/// receives the [`SqliteConnection`] being opened and returns `Ok(())` to
/// continue or an error to fail the open. Use it to register SQL functions,
/// collations, or aggregates through the usual connection API, or to initialize
/// a statically linked C extension such as Spatialite or sqlite-vec via
/// [`SqliteConnection::with_raw_connection`].
///
/// Call this before opening any connection. Extensions run in registration
/// order, and the first error aborts the open. The callback must be a `fn` item
/// or a closure that captures only zero-sized values (enforced at compile
/// time), and registering the same `fn` twice is a no-op. It may run on several
/// threads at once and must not open another connection (which would re-enter
/// the auto-extensions and recurse) or call [`register_auto_extension`],
/// [`cancel_auto_extension`], or [`reset_auto_extension`]. Panics are caught and
/// turned into a failed open.
///
/// [docs]: https://www.sqlite.org/c3ref/auto_extension.html
///
/// # Example
///
/// ```rust
/// use diesel::dsl::sql;
/// use diesel::prelude::*;
/// use diesel::sql_types::Integer;
/// use diesel::sqlite::{register_auto_extension, reset_auto_extension, SqliteConnection};
///
/// // Registers a case-insensitive collation on every new connection.
/// fn my_ext(conn: &mut SqliteConnection) -> QueryResult<()> {
///     conn.register_collation("RUSTNOCASE", |a, b| a.to_lowercase().cmp(&b.to_lowercase()))
/// }
///
/// register_auto_extension(my_ext).unwrap();
///
/// // Every future connection now has the collation.
/// let mut conn = SqliteConnection::establish(":memory:").unwrap();
/// let equal: i32 = sql::<Integer>("SELECT 'a' = 'A' COLLATE RUSTNOCASE")
///     .get_result(&mut conn)
///     .unwrap();
/// assert_eq!(equal, 1);
/// # reset_auto_extension();
/// ```
#[allow(unsafe_code)]
pub fn register_auto_extension<F>(extension: F) -> QueryResult<()>
where
    F: Fn(&mut SqliteConnection) -> QueryResult<()> + Sync + 'static,
{
    let result = unsafe { ffi::sqlite3_auto_extension(Some(entry_point(extension))) };
    if result == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(DatabaseError(
            DatabaseErrorKind::Unknown,
            Box::new(ffi::code_to_str(result).to_string()),
        ))
    }
}

/// Removes a previously registered auto-extension, returning `true` if it was
/// found ([docs][cancel_docs]).
///
/// Pass the same `fn` item given to [`register_auto_extension`]. A closure
/// cannot be cancelled this way, because its type cannot be named again. Use
/// [`reset_auto_extension`] to clear everything instead.
///
/// [cancel_docs]: https://www.sqlite.org/c3ref/cancel_auto_extension.html
#[allow(unsafe_code)]
pub fn cancel_auto_extension<F>(extension: F) -> bool
where
    F: Fn(&mut SqliteConnection) -> QueryResult<()> + Sync + 'static,
{
    unsafe { ffi::sqlite3_cancel_auto_extension(Some(entry_point(extension))) != 0 }
}

/// Clears **all** registered auto-extensions ([docs][reset_docs]).
///
/// After this call, no auto-extensions will run for newly opened connections.
///
/// [reset_docs]: https://www.sqlite.org/c3ref/reset_auto_extension.html
#[allow(unsafe_code)]
pub fn reset_auto_extension() {
    unsafe { ffi::sqlite3_reset_auto_extension() }
}

/// Returns the trampoline for `F`. `extension` is taken by value to infer `F`,
/// then `forget`-ten so the zero-sized callback stays conceptually alive for the
/// process and its destructor never runs.
fn entry_point<F>(extension: F) -> RawAutoExtension
where
    F: Fn(&mut SqliteConnection) -> QueryResult<()> + Sync + 'static,
{
    core::mem::forget(extension);
    trampoline::<F>
}

/// The C entry point handed to SQLite, monomorphized per callback type `F` so
/// each distinct callback maps to a distinct, stable address. SQLite's
/// pointer-based deduplication and [`cancel_auto_extension`] rely on that.
#[allow(unsafe_code)]
unsafe extern "C" fn trampoline<F>(
    db: *mut ffi::sqlite3,
    pz_err_msg: *mut *mut c_char,
    _p_api: *const ffi::sqlite3_api_routines,
) -> c_int
where
    F: Fn(&mut SqliteConnection) -> QueryResult<()> + Sync + 'static,
{
    const {
        assert!(
            core::mem::size_of::<F>() == 0,
            "an auto-extension callback must not capture non-zero-sized state. \
             Use a `fn` item or a closure that captures only zero-sized values"
        );
    }

    // `_p_api` matters only for runtime-loaded shared libraries. Statically
    // linked extensions link the SQLite symbols directly, so we ignore it.
    let result: Result<(), String> =
        crate::util::std_compat::catch_unwind(core::panic::AssertUnwindSafe(|| {
            let Some(db) = core::ptr::NonNull::new(db) else {
                return Err(String::from(
                    "auto-extension received a null database handle",
                ));
            };
            // Reconstruct a *reference* to the zero-sized callback, never an
            // owned value, so its destructor never runs. `&F: Fn` because `F: Fn`.
            // SAFETY: `F` is zero-sized (asserted above), so a dangling, aligned,
            // non-null pointer is a valid `&F`, which `NonNull::dangling` provides.
            let extension: &F = unsafe { core::ptr::NonNull::<F>::dangling().as_ref() };
            // SAFETY: `db` is a valid handle for the duration of this call, and
            // the borrowed connection does not take ownership of it.
            unsafe { SqliteConnection::with_borrowed_connection(db, extension) }
                .map_err(|e| e.to_string())
        }))
        .unwrap_or_else(|panic| {
            Err(match panic_detail(panic) {
                Some(message) => alloc::format!("auto-extension panicked: {message}"),
                None => String::from("auto-extension panicked"),
            })
        });

    match result {
        Ok(()) => ffi::SQLITE_OK,
        Err(message) => {
            set_error_message(pz_err_msg, &message);
            ffi::SQLITE_ERROR
        }
    }
}

/// Best-effort message from a caught panic payload. The no_std `catch_unwind`
/// carries no payload, so only the `std` variant can recover the text.
#[cfg(feature = "std")]
fn panic_detail(panic: alloc::boxed::Box<dyn core::any::Any + Send>) -> Option<String> {
    panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
}

#[cfg(not(feature = "std"))]
fn panic_detail(_panic: ()) -> Option<String> {
    None
}

/// Writes `message` into `*pz_err_msg` with `sqlite3_malloc`, which is the
/// allocator SQLite later frees it with. The message is truncated at the first
/// NUL byte to form a valid C string, and allocation failure is ignored.
#[allow(unsafe_code)]
fn set_error_message(pz_err_msg: *mut *mut c_char, message: &str) {
    if pz_err_msg.is_null() {
        return;
    }

    let bytes = message.as_bytes();
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());

    // SQLite sizes allocations with a C `int`. A message that does not fit is
    // dropped rather than truncated to a bogus length.
    let Ok(size) = c_int::try_from(len + 1) else {
        return;
    };
    let buffer = unsafe { ffi::sqlite3_malloc(size) } as *mut u8;
    if buffer.is_null() {
        return;
    }

    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, len);
        *buffer.add(len) = 0;
        *pz_err_msg = buffer as *mut c_char;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::Integer;
    use std::sync::Mutex;

    // `sqlite3_auto_extension` is process-global, so these tests serialize on
    // this lock and register only benign (never-failing) extensions, leaving
    // connections opened by other tests unaffected. The failing path is covered
    // by `trampoline_maps_result_to_return_code`, which calls the trampoline
    // directly without touching the global registry.
    static AUTO_EXT_TEST_LOCK: Mutex<()> = Mutex::new(());

    // A benign auto-extension: registers a `TESTCOLL` collation through the
    // normal connection API.
    fn test_ext_init(conn: &mut SqliteConnection) -> QueryResult<()> {
        conn.register_collation("TESTCOLL", |a, b| a.cmp(b))
    }

    fn open_memory_connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").expect("Failed to open :memory: connection")
    }

    // Errors out if `TESTCOLL` is not registered on a freshly opened connection.
    fn probe_collation() -> QueryResult<i32> {
        let mut conn = open_memory_connection();
        sql::<Integer>("SELECT 'a' = 'a' COLLATE TESTCOLL").get_result(&mut conn)
    }

    /// RAII guard that calls `reset_auto_extension()` on drop, ensuring global
    /// state is cleaned up even if a test panics.
    struct TestResetGuard;

    impl Drop for TestResetGuard {
        fn drop(&mut self) {
            reset_auto_extension();
        }
    }

    #[test]
    fn auto_extension_lifecycle() {
        let _lock = AUTO_EXT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = TestResetGuard;
        reset_auto_extension();

        // -- 1. register + new connection has the collation --
        register_auto_extension(test_ext_init).unwrap();
        assert_eq!(probe_collation().unwrap(), 1);

        // -- 2. cancel + new connection does NOT have the collation --
        let removed = cancel_auto_extension(test_ext_init);
        assert!(
            removed,
            "cancel should return true for registered extension"
        );
        assert!(
            probe_collation().is_err(),
            "collation should not be available after cancel"
        );

        // -- 3. cancel returns false for unregistered --
        let removed = cancel_auto_extension(test_ext_init);
        assert!(
            !removed,
            "cancel should return false for unregistered extension"
        );

        // -- 4. reset clears all --
        register_auto_extension(test_ext_init).unwrap();
        reset_auto_extension();
        assert!(
            probe_collation().is_err(),
            "collation should not be available after reset"
        );

        // -- 5. duplicate registration is idempotent --
        register_auto_extension(test_ext_init).unwrap();
        register_auto_extension(test_ext_init).unwrap();
        assert_eq!(probe_collation().unwrap(), 1);
        // _guard drops here, ensuring reset even on panic.
    }

    // Drives the trampoline directly (via `entry_point`), without registering
    // it in SQLite's global list, so the Ok/Err/null paths can be checked
    // deterministically without affecting connections opened by other tests.
    #[test]
    #[allow(unsafe_code)]
    fn trampoline_maps_result_to_return_code() {
        fn ok_ext(_conn: &mut SqliteConnection) -> QueryResult<()> {
            Ok(())
        }
        fn err_ext(_conn: &mut SqliteConnection) -> QueryResult<()> {
            Err(Error::QueryBuilderError("boom".into()))
        }

        let ok_tramp = entry_point(ok_ext);
        let err_tramp = entry_point(err_ext);

        let mut conn = open_memory_connection();
        // SAFETY: the pointer is only used for the duration of the closure,
        // while `conn` is alive.
        unsafe {
            conn.with_raw_connection(|db| {
                let mut err: *mut c_char = core::ptr::null_mut();

                // Ok -> SQLITE_OK, no error message allocated.
                let rc = ok_tramp(db, &mut err, core::ptr::null());
                assert_eq!(rc, ffi::SQLITE_OK);
                assert!(err.is_null());

                // Err -> SQLITE_ERROR, message written via sqlite3_malloc.
                let rc = err_tramp(db, &mut err, core::ptr::null());
                assert_eq!(rc, ffi::SQLITE_ERROR);
                assert!(!err.is_null());
                let message = core::ffi::CStr::from_ptr(err)
                    .to_string_lossy()
                    .into_owned();
                assert_eq!(message, "boom");
                ffi::sqlite3_free(err as *mut core::ffi::c_void);

                // Null db handle -> SQLITE_ERROR, never dereferenced.
                let mut err: *mut c_char = core::ptr::null_mut();
                let rc = ok_tramp(core::ptr::null_mut(), &mut err, core::ptr::null());
                assert_eq!(rc, ffi::SQLITE_ERROR);
                if !err.is_null() {
                    ffi::sqlite3_free(err as *mut core::ffi::c_void);
                }
            })
        }
    }

    // Regression test for the closure-reconstruction soundness fix. The callback
    // captures a zero-sized guard with a load-bearing `Drop` that must never run,
    // because the trampoline only reproduces the callback behind a reference and
    // `forget`s the registered value (the old `mem::zeroed()` ran it repeatedly).
    #[test]
    fn callback_zero_sized_capture_is_never_dropped() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static DROPS: AtomicUsize = AtomicUsize::new(0);

        // Zero-sized, `Sync`, with a load-bearing `Drop`.
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::SeqCst);
            }
        }

        let _lock = AUTO_EXT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _reset = TestResetGuard;
        reset_auto_extension();

        let guard = Guard;
        // `move` captures the zero-sized `guard` by value, so the closure is a
        // zero-sized type with a non-trivial destructor.
        register_auto_extension(move |conn: &mut SqliteConnection| {
            let _ = &guard;
            conn.register_collation("TESTCOLL", |a, b| a.cmp(b))
        })
        .unwrap();

        for _ in 0..5 {
            assert_eq!(probe_collation().unwrap(), 1);
        }
        reset_auto_extension();

        assert_eq!(
            DROPS.load(Ordering::SeqCst),
            0,
            "the captured guard's destructor must never run"
        );
    }

    // A panicking callback becomes `SQLITE_ERROR` with the payload recovered
    // into the message (`&str` and `String` payloads, generic fallback
    // otherwise), and drives the panic-unwind path through the drop guard. Gated
    // off WASM, where `catch_unwind` aborts because `panic = "abort"`.
    #[test]
    #[allow(unsafe_code)]
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn trampoline_reports_panic_message() {
        fn panic_str(_conn: &mut SqliteConnection) -> QueryResult<()> {
            panic!("boom-str");
        }
        fn panic_string(_conn: &mut SqliteConnection) -> QueryResult<()> {
            panic!("boom-{}", 7);
        }
        fn panic_other(_conn: &mut SqliteConnection) -> QueryResult<()> {
            std::panic::panic_any(7_u8);
        }

        let cases: [(RawAutoExtension, &str); 3] = [
            (entry_point(panic_str), "auto-extension panicked: boom-str"),
            (entry_point(panic_string), "auto-extension panicked: boom-7"),
            (entry_point(panic_other), "auto-extension panicked"),
        ];

        let mut conn = open_memory_connection();
        // SAFETY: the pointer is only used while `conn` is alive.
        unsafe {
            conn.with_raw_connection(|db| {
                for (tramp, expected) in cases {
                    let mut err: *mut c_char = core::ptr::null_mut();
                    let rc = tramp(db, &mut err, core::ptr::null());
                    assert_eq!(rc, ffi::SQLITE_ERROR);
                    assert!(!err.is_null());
                    let message = core::ffi::CStr::from_ptr(err)
                        .to_string_lossy()
                        .into_owned();
                    assert_eq!(message, expected);
                    ffi::sqlite3_free(err as *mut core::ffi::c_void);
                }
            })
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn error_message_truncates_at_interior_nul() {
        let mut err: *mut c_char = core::ptr::null_mut();
        set_error_message(&mut err, "before\0after");
        assert!(!err.is_null());
        // SAFETY: `err` is a sqlite-allocated C string we own until we free it.
        unsafe {
            let truncated = core::ffi::CStr::from_ptr(err).to_str().unwrap();
            assert_eq!(truncated, "before");
            ffi::sqlite3_free(err as *mut core::ffi::c_void);
        }

        // A null out-pointer is a no-op (must not write through it or crash).
        set_error_message(core::ptr::null_mut(), "ignored");
    }
}
