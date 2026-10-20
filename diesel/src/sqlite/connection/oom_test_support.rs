//! Test support for forcing SQLite allocation failures.

use super::ffi;
use alloc::string::String;

const CHILD_ENV: &str = "DIESEL_SQLITE_OOM_CHILD";

struct HardHeapLimit(i64);

impl Drop for HardHeapLimit {
    fn drop(&mut self) {
        // SAFETY: SQLite accepts every i64 as a limit and reads no Rust memory.
        unsafe {
            ffi::sqlite3_hard_heap_limit64(self.0);
        }
    }
}

/// Rejects every SQLite allocation beyond `spare` bytes while `f` runs.
pub(super) fn with_heap_limit<R>(spare: i64, f: impl FnOnce() -> R) -> R {
    // SAFETY: These process global SQLite APIs read no Rust memory.
    let previous = unsafe {
        let current = ffi::sqlite3_memory_used();
        ffi::sqlite3_hard_heap_limit64(current + spare)
    };
    let _guard = HardHeapLimit(previous);
    f()
}

/// Reruns the calling test in a child process to isolate global state and returns its stderr.
pub(super) fn run_in_child(f: impl FnOnce()) -> String {
    let current_thread = std::thread::current();
    let test_name = current_thread
        .name()
        .expect("the test harness names every test thread");
    if std::env::var_os(CHILD_ENV).as_deref() == Some(std::ffi::OsStr::new(test_name)) {
        f();
        return String::new();
    }

    let output =
        std::process::Command::new(std::env::current_exe().expect("the test binary has a path"))
            .arg("--exact")
            .arg(test_name)
            .arg("--nocapture")
            .env(CHILD_ENV, test_name)
            .output()
            .expect("the child test process starts");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "child test failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // libtest exits successfully when an exact filter matches no tests.
    assert!(
        stdout.contains("test result: ok. 1 passed"),
        "child test did not run exactly one test\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    stderr
}

pub(super) fn panic_message(payload: &(dyn core::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("the panic payload is a string")
}
