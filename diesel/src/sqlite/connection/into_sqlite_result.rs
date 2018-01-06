extern crate libsqlite3_sys as ffi;

use std::ffi::{CString, CStr};
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

unsafe extern "C" fn free_cstring(p: *mut ::std::os::raw::c_void) {
    let _: CString = CString::from_raw(::std::mem::transmute(p));
}

impl IntoSqliteResult for CString {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        // TODO Narrowing typecast, could overflow.
        //  * sqlite3_result_text64 accepts 64 bit len, but is not exposed
        //    by ffi library
        //  * Never the less, the cast should be checked for overflow
        // See also CStr-implementation below
        let len = self.as_bytes().len() as libc::c_int;

        unsafe {
            ffi::sqlite3_result_text(
                ctx,
                self.into_raw(),
                len,
                Some(free_cstring),
            )
        }
    }
}

impl IntoSqliteResult for String {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        CString::new(self)
            .expect("TODO Missing error propagation")
            .into_sqlite_result(ctx)
    }
}

impl IntoSqliteResult for &'static CStr {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        // See note in CString-implementation above about cast
        let len = self.to_bytes().len() as libc::c_int;

        unsafe {
            ffi::sqlite3_result_text(
                ctx,
                self.as_ptr(),
                len,
                ffi::SQLITE_STATIC(),
            )
        }
    }
}
