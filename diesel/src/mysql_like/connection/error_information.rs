#![allow(unsafe_code)] // module uses ffi
use core::ffi::{CStr, c_char};

use crate::result::{DatabaseErrorInformation, SqlState};

pub(super) struct MysqlLikeErrorInformation {
    pub(super) message: String,
    pub(super) sqlstate: Option<SqlState>,
}

/// Reads the result of `mysql_sqlstate` or `mysql_stmt_sqlstate`.
///
/// # Safety
///
/// `sqlstate` must be null or point to a NUL terminated buffer that stays
/// unchanged for the duration of the call.
pub(super) unsafe fn sqlstate_from_ptr(sqlstate: *const c_char) -> Option<SqlState> {
    if sqlstate.is_null() {
        return None;
    }
    // SAFETY: non-null, and the caller guarantees NUL termination and stability.
    let bytes = unsafe { CStr::from_ptr(sqlstate) }.to_bytes();
    <[u8; 5]>::try_from(bytes).ok().and_then(SqlState::new)
}

impl DatabaseErrorInformation for MysqlLikeErrorInformation {
    fn message(&self) -> &str {
        &self.message
    }

    fn details(&self) -> Option<&str> {
        None
    }

    fn hint(&self) -> Option<&str> {
        None
    }

    fn table_name(&self) -> Option<&str> {
        None
    }

    fn column_name(&self) -> Option<&str> {
        None
    }

    fn constraint_name(&self) -> Option<&str> {
        None
    }

    fn statement_position(&self) -> Option<i32> {
        None
    }

    fn sqlstate(&self) -> Option<SqlState> {
        self.sqlstate
    }
}
