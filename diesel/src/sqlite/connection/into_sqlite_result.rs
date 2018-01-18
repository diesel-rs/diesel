use std::ffi::{CString, CStr};
use std::os::raw as libc;

use super::ffi;

// TODO: Support BLOB values

pub trait IntoSqliteResult {
    /// `ctx` must be a valid pointer
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context);
}

pub trait IntoSqliteResultError {
    /// `ctx` must be a valid pointer
    fn into_sqlite_result_error(self, ctx: *mut ffi::sqlite3_context);
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

impl<T: IntoSqliteResult> IntoSqliteResult for Option<T> {
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        match self {
            Some(t) => t.into_sqlite_result(ctx),
            None => unsafe {
                ffi::sqlite3_result_null(ctx)
            },
        }
    }
}

#[warn(missing_docs)] //FIXME
pub mod error {
    use super::ffi;
    use super::IntoSqliteResultError;

    #[derive(Debug, Copy, Clone)]
    pub struct TooBig;
    impl IntoSqliteResultError for TooBig {
        fn into_sqlite_result_error(self, ctx: *mut ffi::sqlite3_context) {
            unsafe {
                ffi::sqlite3_result_error_toobig(ctx)
            }
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct NoMem;
    impl IntoSqliteResultError for NoMem {
        fn into_sqlite_result_error(self, ctx: *mut ffi::sqlite3_context) {
            unsafe {
                ffi::sqlite3_result_error_nomem(ctx)
            }
        }
    }

    #[derive(Debug)]
    pub struct Text(pub ::std::ffi::CString);
    impl IntoSqliteResultError for Text {
        fn into_sqlite_result_error(self, ctx: *mut ffi::sqlite3_context) {
            unsafe {
                ffi::sqlite3_result_error(ctx, self.0.as_ptr(), -1)
            }
        }
    }
}

impl<T, E> IntoSqliteResult for Result<T, E>
where
    T: IntoSqliteResult,
    E: IntoSqliteResultError,
{
    fn into_sqlite_result(self, ctx: *mut ffi::sqlite3_context) {
        match self {
            Ok(t) => t.into_sqlite_result(ctx),
            Err(e) => e.into_sqlite_result_error(ctx),
        }
    }
}
