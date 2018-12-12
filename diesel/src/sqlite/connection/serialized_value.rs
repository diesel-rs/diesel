extern crate libsqlite3_sys as ffi;

use std::os::raw as libc;
use std::ptr::{self, NonNull};

use sqlite::SqliteType;

pub struct SerializedValue {
    pub ty: SqliteType,
    pub data: Option<Vec<u8>>,
}

impl SerializedValue {
    // We are always reading potentially misaligned pointers with
    // `ptr::read_unaligned`
    #[allow(clippy::cast_ptr_alignment)]
    pub(crate) fn bind_to(self, stmt: NonNull<ffi::sqlite3_stmt>, idx: libc::c_int) -> libc::c_int {
        // This unsafe block assumes the following invariants:
        //
        // - `stmt` points to valid memory
        // - If `self.ty` is anything other than `Binary` or `Text`, the appropriate
        //   number of bytes were written to `value` for an integer of the
        //   corresponding size.
        unsafe {
            match (self.ty, self.data) {
                (_, None) => ffi::sqlite3_bind_null(stmt.as_ptr(), idx),
                (SqliteType::Binary, Some(bytes)) => ffi::sqlite3_bind_blob(
                    stmt.as_ptr(),
                    idx,
                    bytes.as_ptr() as *const libc::c_void,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                (SqliteType::Text, Some(bytes)) => ffi::sqlite3_bind_text(
                    stmt.as_ptr(),
                    idx,
                    bytes.as_ptr() as *const libc::c_char,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                (SqliteType::Float, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const f32);
                    ffi::sqlite3_bind_double(stmt.as_ptr(), idx, libc::c_double::from(value))
                }
                (SqliteType::Double, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const f64);
                    ffi::sqlite3_bind_double(stmt.as_ptr(), idx, value as libc::c_double)
                }
                (SqliteType::SmallInt, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const i16);
                    ffi::sqlite3_bind_int(stmt.as_ptr(), idx, libc::c_int::from(value))
                }
                (SqliteType::Integer, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const i32);
                    ffi::sqlite3_bind_int(stmt.as_ptr(), idx, value as libc::c_int)
                }
                (SqliteType::Long, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const i64);
                    ffi::sqlite3_bind_int64(stmt.as_ptr(), idx, value)
                }
            }
        }
    }

    // We are always reading potentially misaligned pointers with
    // `ptr::read_unaligned`
    #[allow(clippy::cast_ptr_alignment)]
    pub fn result_of(self, ctx: *mut ffi::sqlite3_context) {
        // This unsafe block assumes the following invariants:
        //
        // - `ctx` points to valid memory
        // - If `self.ty` is anything other than `Binary` or `Text`, the appropriate
        //   number of bytes were written to `self.data` for an integer of the
        //   corresponding size.
        unsafe {
            match (self.ty, self.data) {
                (_, None) => ffi::sqlite3_result_null(ctx),
                (SqliteType::Binary, Some(bytes)) => ffi::sqlite3_result_blob(
                    ctx,
                    bytes.as_ptr() as *const libc::c_void,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                (SqliteType::Text, Some(bytes)) => ffi::sqlite3_result_text(
                    ctx,
                    bytes.as_ptr() as *const libc::c_char,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                (SqliteType::Float, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const f32);
                    ffi::sqlite3_result_double(ctx, libc::c_double::from(value))
                }
                (SqliteType::Double, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const f64);
                    ffi::sqlite3_result_double(ctx, value as libc::c_double)
                }
                (SqliteType::SmallInt, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const i16);
                    ffi::sqlite3_result_int(ctx, libc::c_int::from(value))
                }
                (SqliteType::Integer, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const i32);
                    ffi::sqlite3_result_int(ctx, value as libc::c_int)
                }
                (SqliteType::Long, Some(bytes)) => {
                    let value = ptr::read_unaligned(bytes.as_ptr() as *const i64);
                    ffi::sqlite3_result_int64(ctx, value)
                }
            }
        }
    }
}
