extern crate libsqlite3_sys as ffi;

use std::os::raw as libc;

pub trait IntoSqliteResult {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context);
}

impl IntoSqliteResult for i32 {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        unsafe {
            ffi::sqlite3_result_int(ctx, self);
        }
    }
}

impl IntoSqliteResult for i64 {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        unsafe {
            ffi::sqlite3_result_int64(ctx, self);
        }
    }
}

impl IntoSqliteResult for f64 {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        unsafe {
            ffi::sqlite3_result_double(ctx, self);
        }
    }
}

impl IntoSqliteResult for String {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        // Inefficient. Assumes ownership of the String, and also tells SQLite
        // to copy it (SQLITE_TRANSIENT). We could send a destructor as the
        // last argument to sqlite3_result_text and give ownership of the
        // string to SQLite.

        // BUG: Will allow sending strings with embedded NUL characters into
        // SQLite. Working with strings with embedded NUL characters causes
        // undefined behavior in SQLite.
        unsafe {
            ffi::sqlite3_result_text(
                ctx,
                self.as_ptr() as *const libc::c_char,
                self.len() as libc::c_int,
                ffi::SQLITE_TRANSIENT(),
            )
        }
    }
}

impl IntoSqliteResult for &'static str {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        // BUG: Will allow sending strings with embedded NUL characters into
        // SQLite. Working with strings with embedded NUL characters causes
        // undefined behavior in SQLite.
        unsafe {
            ffi::sqlite3_result_text(
                ctx,
                self.as_ptr() as *const libc::c_char,
                self.len() as libc::c_int,
                ffi::SQLITE_STATIC(),
            )
        }
    }
}
