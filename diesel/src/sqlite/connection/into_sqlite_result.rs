extern crate libsqlite3_sys as ffi;

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
