//! Types for the SQLite authorizer callback.
//!
//! See [`sqlite3_set_authorizer`](https://sqlite.org/c3ref/set_authorizer.html)
//! and the [action code constants](https://sqlite.org/c3ref/c_alter_table.html).

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

/// Authorizer action codes.
///
/// These correspond to the action codes passed to the authorizer callback
/// by SQLite. The callback is invoked during SQL statement compilation
/// (not during execution).
///
/// Added in SQLite 3.0.0 (June 2004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthorizerAction {
    /// CREATE INDEX
    CreateIndex,
    /// CREATE TABLE
    CreateTable,
    /// CREATE TEMP INDEX
    CreateTempIndex,
    /// CREATE TEMP TABLE
    CreateTempTable,
    /// CREATE TEMP TRIGGER
    CreateTempTrigger,
    /// CREATE TEMP VIEW
    CreateTempView,
    /// CREATE TRIGGER
    CreateTrigger,
    /// CREATE VIEW
    CreateView,
    /// DELETE
    Delete,
    /// DROP INDEX
    DropIndex,
    /// DROP TABLE
    DropTable,
    /// DROP TEMP INDEX
    DropTempIndex,
    /// DROP TEMP TABLE
    DropTempTable,
    /// DROP TEMP TRIGGER
    DropTempTrigger,
    /// DROP TEMP VIEW
    DropTempView,
    /// DROP TRIGGER
    DropTrigger,
    /// DROP VIEW
    DropView,
    /// INSERT
    Insert,
    /// PRAGMA
    Pragma,
    /// Read a column value
    Read,
    /// SELECT statement
    Select,
    /// BEGIN/COMMIT/ROLLBACK
    Transaction,
    /// UPDATE
    Update,
    /// ATTACH DATABASE
    Attach,
    /// DETACH DATABASE
    Detach,
    /// ALTER TABLE
    AlterTable,
    /// REINDEX
    Reindex,
    /// ANALYZE
    Analyze,
    /// CREATE VIRTUAL TABLE
    CreateVTable,
    /// DROP VIRTUAL TABLE
    DropVTable,
    /// SQL function call
    Function,
    /// SAVEPOINT
    Savepoint,
    /// Recursive SELECT
    Recursive,
    /// Unknown action code (for future compatibility)
    Unknown(i32),
}

impl AuthorizerAction {
    /// Returns `true` if this action modifies the database schema.
    ///
    /// Schema-modifying actions include creating, dropping, or altering
    /// tables, indexes, triggers, views, and virtual tables.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::sqlite::AuthorizerAction;
    ///
    /// assert!(AuthorizerAction::CreateTable.is_schema_modifying());
    /// assert!(AuthorizerAction::DropIndex.is_schema_modifying());
    /// assert!(AuthorizerAction::AlterTable.is_schema_modifying());
    ///
    /// assert!(!AuthorizerAction::Select.is_schema_modifying());
    /// assert!(!AuthorizerAction::Insert.is_schema_modifying());
    /// assert!(!AuthorizerAction::Update.is_schema_modifying());
    /// assert!(!AuthorizerAction::Delete.is_schema_modifying());
    /// ```
    pub fn is_schema_modifying(&self) -> bool {
        matches!(
            self,
            Self::CreateIndex
                | Self::CreateTable
                | Self::CreateTempIndex
                | Self::CreateTempTable
                | Self::CreateTempTrigger
                | Self::CreateTempView
                | Self::CreateTrigger
                | Self::CreateView
                | Self::CreateVTable
                | Self::DropIndex
                | Self::DropTable
                | Self::DropTempIndex
                | Self::DropTempTable
                | Self::DropTempTrigger
                | Self::DropTempView
                | Self::DropTrigger
                | Self::DropView
                | Self::DropVTable
                | Self::AlterTable
        )
    }

    /// Converts an FFI action code to the Rust enum variant.
    pub(crate) fn from_ffi(code: i32) -> Self {
        match code {
            ffi::SQLITE_CREATE_INDEX => Self::CreateIndex,
            ffi::SQLITE_CREATE_TABLE => Self::CreateTable,
            ffi::SQLITE_CREATE_TEMP_INDEX => Self::CreateTempIndex,
            ffi::SQLITE_CREATE_TEMP_TABLE => Self::CreateTempTable,
            ffi::SQLITE_CREATE_TEMP_TRIGGER => Self::CreateTempTrigger,
            ffi::SQLITE_CREATE_TEMP_VIEW => Self::CreateTempView,
            ffi::SQLITE_CREATE_TRIGGER => Self::CreateTrigger,
            ffi::SQLITE_CREATE_VIEW => Self::CreateView,
            ffi::SQLITE_DELETE => Self::Delete,
            ffi::SQLITE_DROP_INDEX => Self::DropIndex,
            ffi::SQLITE_DROP_TABLE => Self::DropTable,
            ffi::SQLITE_DROP_TEMP_INDEX => Self::DropTempIndex,
            ffi::SQLITE_DROP_TEMP_TABLE => Self::DropTempTable,
            ffi::SQLITE_DROP_TEMP_TRIGGER => Self::DropTempTrigger,
            ffi::SQLITE_DROP_TEMP_VIEW => Self::DropTempView,
            ffi::SQLITE_DROP_TRIGGER => Self::DropTrigger,
            ffi::SQLITE_DROP_VIEW => Self::DropView,
            ffi::SQLITE_INSERT => Self::Insert,
            ffi::SQLITE_PRAGMA => Self::Pragma,
            ffi::SQLITE_READ => Self::Read,
            ffi::SQLITE_SELECT => Self::Select,
            ffi::SQLITE_TRANSACTION => Self::Transaction,
            ffi::SQLITE_UPDATE => Self::Update,
            ffi::SQLITE_ATTACH => Self::Attach,
            ffi::SQLITE_DETACH => Self::Detach,
            ffi::SQLITE_ALTER_TABLE => Self::AlterTable,
            ffi::SQLITE_REINDEX => Self::Reindex,
            ffi::SQLITE_ANALYZE => Self::Analyze,
            ffi::SQLITE_CREATE_VTABLE => Self::CreateVTable,
            ffi::SQLITE_DROP_VTABLE => Self::DropVTable,
            ffi::SQLITE_FUNCTION => Self::Function,
            ffi::SQLITE_SAVEPOINT => Self::Savepoint,
            ffi::SQLITE_RECURSIVE => Self::Recursive,
            other => Self::Unknown(other),
        }
    }
}

/// Context information passed to the authorizer callback.
///
/// The meaning of the string arguments depends on the action code:
///
/// | Action | arg1 | arg2 | db_name | accessor |
/// |--------|------|------|---------|----------|
/// | `CreateIndex` | Index name | Table name | db | - |
/// | `CreateTable` | Table name | - | db | - |
/// | `CreateTrigger` | Trigger name | Table name | db | - |
/// | `CreateView` | View name | - | db | - |
/// | `Delete` | Table name | - | db | - |
/// | `DropIndex` | Index name | Table name | db | - |
/// | `DropTable` | Table name | - | db | - |
/// | `DropTrigger` | Trigger name | Table name | db | - |
/// | `DropView` | View name | - | db | - |
/// | `Insert` | Table name | - | db | - |
/// | `Pragma` | Pragma name | Argument | db | - |
/// | `Read` | Table name | Column name | db | trigger/view |
/// | `Select` | - | - | - | - |
/// | `Transaction` | Operation | - | - | - |
/// | `Update` | Table name | Column name | db | - |
/// | `Attach` | Filename | - | - | - |
/// | `Detach` | Database name | - | - | - |
/// | `AlterTable` | Database name | Table name | - | - |
/// | `Reindex` | Index name | - | db | - |
/// | `Analyze` | Table name | - | db | - |
/// | `CreateVTable` | Table name | Module name | db | - |
/// | `DropVTable` | Table name | Module name | db | - |
/// | `Function` | - | Function name | - | - |
/// | `Savepoint` | Operation | Name | - | - |
/// | `Recursive` | - | - | - | - |
///
/// Where "db" means `"main"`, `"temp"`, or an `ATTACH` alias.
/// The "accessor" field indicates which trigger or view is causing
/// the access, if applicable.
#[derive(Debug, Clone, Copy)]
pub struct AuthorizerContext<'a> {
    /// The action being authorized.
    pub action: AuthorizerAction,
    /// First argument (meaning depends on action).
    pub arg1: Option<&'a str>,
    /// Second argument (meaning depends on action).
    pub arg2: Option<&'a str>,
    /// Database name (`"main"`, `"temp"`, or `ATTACH` alias).
    pub db_name: Option<&'a str>,
    /// Trigger or view name causing the access (if applicable).
    pub accessor: Option<&'a str>,
}

/// Authorizer callback decision.
///
/// Returned by the authorizer callback to indicate whether to allow,
/// ignore, or deny the requested operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizerDecision {
    /// Allow the operation to proceed (`SQLITE_OK`).
    Allow,
    /// Ignore the operation (`SQLITE_IGNORE`).
    ///
    /// For `Read` actions, returns `NULL` instead of the column value.
    /// For other actions, treats the operation as a no-op.
    Ignore,
    /// Deny the operation (`SQLITE_DENY`).
    ///
    /// Causes the entire SQL statement to fail with an authorization error.
    Deny,
}

impl AuthorizerDecision {
    /// Converts the decision to the FFI return code.
    pub(crate) fn to_ffi(self) -> i32 {
        match self {
            Self::Allow => ffi::SQLITE_OK,
            Self::Ignore => ffi::SQLITE_IGNORE,
            Self::Deny => ffi::SQLITE_DENY,
        }
    }
}
