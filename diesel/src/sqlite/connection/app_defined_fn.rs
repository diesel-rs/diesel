//! Plumbing for the implementation of the application defined function API

use std::os::raw as libc;

use super::ffi;
use super::into_sqlite_result::IntoSqliteResult;
use super::from_sqlite_value::FromSqliteValue;

/// Context is a wrapper for the SQLite function evaluation context.
#[derive(Debug)]
pub struct Context<'a> {
    ctx: *mut ffi::sqlite3_context,
    args: &'a [*mut ffi::sqlite3_value],
}

// Context is translated from rusqlite
impl<'a> Context<'a> {
    /// Returns the number of arguments to the function.
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Returns `true` when there is no argument.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// Returns the `idx`th argument as a `T`.
    ///
    /// # Failure
    ///
    /// Will panic if `idx` is greater than or equal to `self.len()`.
    pub fn get<T>(&self, idx: usize) -> T
    where
        T: FromSqliteValue
    {
        let arg = self.args[idx];

        T::from_sqlite_value(arg)
    }
}

pub unsafe extern "C" fn free_boxed_value<T>(p: *mut ::std::os::raw::c_void) {
    let _: Box<T> = Box::from_raw(::std::mem::transmute(p));
}

pub unsafe extern "C" fn call_boxed_closure<F, T>(
    ctx: *mut ffi::sqlite3_context,
    argc: libc::c_int,
    argv: *mut *mut ffi::sqlite3_value
)
where
    F: FnMut(&Context) -> T,
    T: IntoSqliteResult
{
    use std::{slice, mem};

    let ctx = Context {
        ctx: ctx,
        args: slice::from_raw_parts(argv, argc as usize),
    };

    let boxed_f: *mut F = mem::transmute(ffi::sqlite3_user_data(ctx.ctx));
    assert!(!boxed_f.is_null(), "Internal error - null function pointer");

    let t = (*boxed_f)(&ctx);

    t.into_sqlite_result(ctx.ctx);
}
