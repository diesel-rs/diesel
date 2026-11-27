use crate::result::DatabaseErrorInformation;
use alloc::string::String;

/// Error information for a failed SQLite API call.
pub(in crate::sqlite) struct SqliteErrorInformation {
    pub(in crate::sqlite) message: String,
    pub(in crate::sqlite) native_code: i32,
}

impl DatabaseErrorInformation for SqliteErrorInformation {
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

    fn native_code(&self) -> Option<i32> {
        Some(self.native_code)
    }
}
