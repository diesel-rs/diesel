//! Types for the SQLite authorizer callback.
//!
//! See [`sqlite3_set_authorizer`](https://sqlite.org/c3ref/set_authorizer.html)
//! and the [action code constants](https://sqlite.org/c3ref/c_alter_table.html).
//!
//! The authorizer callback receives an [`AuthorizerContext`]. It is an enum
//! with one variant per SQLite action code, and every variant carries the
//! arguments for that action under descriptive field names, so there is no
//! need to consult a table of what each positional argument means.
//!
//! Two fields recur across variants. `database` is the name of the database
//! (`"main"`, `"temp"`, or an `ATTACH` alias) an object lives in, when SQLite
//! reports it. `accessor` is the name of the inner-most trigger or view
//! responsible for the access, or `None` when the access is directly from
//! top-level SQL. A string argument is `None` when SQLite passes `NULL` or a
//! value that is not valid UTF-8.

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

/// Authorizing a `CREATE INDEX` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateIndex<'a> {
    /// Name of the index being created.
    pub index: Option<&'a str>,
    /// Name of the table the index is created on.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateTable<'a> {
    /// Name of the table being created.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE TEMP INDEX` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateTempIndex<'a> {
    /// Name of the index being created.
    pub index: Option<&'a str>,
    /// Name of the table the index is created on.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE TEMP TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateTempTable<'a> {
    /// Name of the table being created.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE TEMP TRIGGER` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateTempTrigger<'a> {
    /// Name of the trigger being created.
    pub trigger: Option<&'a str>,
    /// Name of the table the trigger is attached to.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE TEMP VIEW` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateTempView<'a> {
    /// Name of the view being created.
    pub view: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE TRIGGER` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateTrigger<'a> {
    /// Name of the trigger being created.
    pub trigger: Option<&'a str>,
    /// Name of the table the trigger is attached to.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE VIEW` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateView<'a> {
    /// Name of the view being created.
    pub view: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DELETE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Delete<'a> {
    /// Name of the table rows are deleted from.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP INDEX` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropIndex<'a> {
    /// Name of the index being dropped.
    pub index: Option<&'a str>,
    /// Name of the table the index belongs to.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTable<'a> {
    /// Name of the table being dropped.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP TEMP INDEX` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTempIndex<'a> {
    /// Name of the index being dropped.
    pub index: Option<&'a str>,
    /// Name of the table the index belongs to.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP TEMP TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTempTable<'a> {
    /// Name of the table being dropped.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP TEMP TRIGGER` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTempTrigger<'a> {
    /// Name of the trigger being dropped.
    pub trigger: Option<&'a str>,
    /// Name of the table the trigger is attached to.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP TEMP VIEW` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTempView<'a> {
    /// Name of the view being dropped.
    pub view: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP TRIGGER` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTrigger<'a> {
    /// Name of the trigger being dropped.
    pub trigger: Option<&'a str>,
    /// Name of the table the trigger is attached to.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP VIEW` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropView<'a> {
    /// Name of the view being dropped.
    pub view: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing an `INSERT` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Insert<'a> {
    /// Name of the table rows are inserted into.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `PRAGMA` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Pragma<'a> {
    /// Name of the pragma.
    pub name: Option<&'a str>,
    /// Argument to the pragma, or `None` when it is queried without one.
    pub argument: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a column read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Read<'a> {
    /// Name of the table being read.
    pub table: Option<&'a str>,
    /// Name of the column being read.
    pub column: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `SELECT` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Select<'a> {
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a transaction control operation (`BEGIN`, `COMMIT`, `ROLLBACK`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Transaction<'a> {
    /// The operation, for example `"BEGIN"`, `"COMMIT"`, or `"ROLLBACK"`.
    pub operation: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing an `UPDATE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Update<'a> {
    /// Name of the table being updated.
    pub table: Option<&'a str>,
    /// Name of the column being updated.
    pub column: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing an `ATTACH DATABASE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Attach<'a> {
    /// Filename of the database being attached.
    pub filename: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DETACH DATABASE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Detach<'a> {
    /// Name of the database being detached.
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing an `ALTER TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AlterTable<'a> {
    /// Database the table lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Name of the table being altered.
    pub table: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `REINDEX` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Reindex<'a> {
    /// Name of the index being rebuilt.
    pub index: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing an `ANALYZE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Analyze<'a> {
    /// Name of the table being analyzed.
    pub table: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `CREATE VIRTUAL TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CreateVTable<'a> {
    /// Name of the virtual table being created.
    pub table: Option<&'a str>,
    /// Name of the module implementing the virtual table.
    pub module: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `DROP VIRTUAL TABLE` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropVTable<'a> {
    /// Name of the virtual table being dropped.
    pub table: Option<&'a str>,
    /// Name of the module implementing the virtual table.
    pub module: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a SQL function call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Function<'a> {
    /// Name of the function being called.
    pub function: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a `SAVEPOINT` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Savepoint<'a> {
    /// The operation, for example `"BEGIN"`, `"RELEASE"`, or `"ROLLBACK"`.
    pub operation: Option<&'a str>,
    /// Name of the savepoint.
    pub name: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Authorizing a recursive `SELECT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Recursive<'a> {
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// An action code SQLite reported that this version of diesel does not model.
///
/// The raw arguments are exposed unchanged for forward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Unknown<'a> {
    /// The raw SQLite action code.
    pub code: i32,
    /// The third argument to the authorizer callback, if any.
    pub arg1: Option<&'a str>,
    /// The fourth argument to the authorizer callback, if any.
    pub arg2: Option<&'a str>,
    /// Database the object lives in (`"main"`, `"temp"`, or an `ATTACH` alias).
    pub database: Option<&'a str>,
    /// Inner-most trigger or view responsible for the access, if any.
    pub accessor: Option<&'a str>,
}

/// Context information passed to the authorizer callback.
///
/// One variant per SQLite action code. Each variant carries the arguments for
/// that action under descriptive field names, so there is no need to consult a
/// table of what each positional argument means. The callback is invoked
/// during SQL statement compilation (not during execution).
///
/// Added in SQLite 3.0.0 (June 2004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthorizerContext<'a> {
    /// `CREATE INDEX`
    CreateIndex(CreateIndex<'a>),
    /// `CREATE TABLE`
    CreateTable(CreateTable<'a>),
    /// `CREATE TEMP INDEX`
    CreateTempIndex(CreateTempIndex<'a>),
    /// `CREATE TEMP TABLE`
    CreateTempTable(CreateTempTable<'a>),
    /// `CREATE TEMP TRIGGER`
    CreateTempTrigger(CreateTempTrigger<'a>),
    /// `CREATE TEMP VIEW`
    CreateTempView(CreateTempView<'a>),
    /// `CREATE TRIGGER`
    CreateTrigger(CreateTrigger<'a>),
    /// `CREATE VIEW`
    CreateView(CreateView<'a>),
    /// `DELETE`
    Delete(Delete<'a>),
    /// `DROP INDEX`
    DropIndex(DropIndex<'a>),
    /// `DROP TABLE`
    DropTable(DropTable<'a>),
    /// `DROP TEMP INDEX`
    DropTempIndex(DropTempIndex<'a>),
    /// `DROP TEMP TABLE`
    DropTempTable(DropTempTable<'a>),
    /// `DROP TEMP TRIGGER`
    DropTempTrigger(DropTempTrigger<'a>),
    /// `DROP TEMP VIEW`
    DropTempView(DropTempView<'a>),
    /// `DROP TRIGGER`
    DropTrigger(DropTrigger<'a>),
    /// `DROP VIEW`
    DropView(DropView<'a>),
    /// `INSERT`
    Insert(Insert<'a>),
    /// `PRAGMA`
    Pragma(Pragma<'a>),
    /// Reading a column value.
    Read(Read<'a>),
    /// `SELECT`
    Select(Select<'a>),
    /// Transaction control (`BEGIN`, `COMMIT`, `ROLLBACK`).
    Transaction(Transaction<'a>),
    /// `UPDATE`
    Update(Update<'a>),
    /// `ATTACH DATABASE`
    Attach(Attach<'a>),
    /// `DETACH DATABASE`
    Detach(Detach<'a>),
    /// `ALTER TABLE`
    AlterTable(AlterTable<'a>),
    /// `REINDEX`
    Reindex(Reindex<'a>),
    /// `ANALYZE`
    Analyze(Analyze<'a>),
    /// `CREATE VIRTUAL TABLE`
    CreateVTable(CreateVTable<'a>),
    /// `DROP VIRTUAL TABLE`
    DropVTable(DropVTable<'a>),
    /// A SQL function call.
    Function(Function<'a>),
    /// `SAVEPOINT`
    Savepoint(Savepoint<'a>),
    /// A recursive `SELECT`.
    Recursive(Recursive<'a>),
    /// An action code this version of diesel does not model.
    Unknown(Unknown<'a>),
}

impl<'a> AuthorizerContext<'a> {
    /// Returns `true` if this action modifies the database schema.
    ///
    /// Schema-modifying actions include creating, dropping, or altering
    /// tables, indexes, triggers, views, and virtual tables.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, AuthorizerContext, AuthorizerDecision};
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_authorize(|ctx| {
    ///     if ctx.is_schema_modifying() {
    ///         AuthorizerDecision::Deny
    ///     } else {
    ///         AuthorizerDecision::Allow
    ///     }
    /// });
    /// ```
    pub fn is_schema_modifying(&self) -> bool {
        matches!(
            self,
            Self::CreateIndex(_)
                | Self::CreateTable(_)
                | Self::CreateTempIndex(_)
                | Self::CreateTempTable(_)
                | Self::CreateTempTrigger(_)
                | Self::CreateTempView(_)
                | Self::CreateTrigger(_)
                | Self::CreateView(_)
                | Self::CreateVTable(_)
                | Self::DropIndex(_)
                | Self::DropTable(_)
                | Self::DropTempIndex(_)
                | Self::DropTempTable(_)
                | Self::DropTempTrigger(_)
                | Self::DropTempView(_)
                | Self::DropTrigger(_)
                | Self::DropView(_)
                | Self::DropVTable(_)
                | Self::AlterTable(_)
        )
    }

    /// Builds the context from the raw authorizer callback arguments.
    ///
    /// `database` is the 5th callback argument (the database name) and
    /// `accessor` is the 6th (the trigger or view responsible). The `arg1` and
    /// `arg2` values are the 3rd and 4th arguments, whose meaning depends on
    /// the action code per the SQLite documentation.
    pub(crate) fn from_ffi(
        code: i32,
        arg1: Option<&'a str>,
        arg2: Option<&'a str>,
        database: Option<&'a str>,
        accessor: Option<&'a str>,
    ) -> Self {
        match code {
            ffi::SQLITE_CREATE_INDEX => Self::CreateIndex(CreateIndex {
                index: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_TABLE => Self::CreateTable(CreateTable {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_TEMP_INDEX => Self::CreateTempIndex(CreateTempIndex {
                index: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_TEMP_TABLE => Self::CreateTempTable(CreateTempTable {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_TEMP_TRIGGER => Self::CreateTempTrigger(CreateTempTrigger {
                trigger: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_TEMP_VIEW => Self::CreateTempView(CreateTempView {
                view: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_TRIGGER => Self::CreateTrigger(CreateTrigger {
                trigger: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_VIEW => Self::CreateView(CreateView {
                view: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_DELETE => Self::Delete(Delete {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_INDEX => Self::DropIndex(DropIndex {
                index: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_TABLE => Self::DropTable(DropTable {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_TEMP_INDEX => Self::DropTempIndex(DropTempIndex {
                index: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_TEMP_TABLE => Self::DropTempTable(DropTempTable {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_TEMP_TRIGGER => Self::DropTempTrigger(DropTempTrigger {
                trigger: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_TEMP_VIEW => Self::DropTempView(DropTempView {
                view: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_TRIGGER => Self::DropTrigger(DropTrigger {
                trigger: arg1,
                table: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_VIEW => Self::DropView(DropView {
                view: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_INSERT => Self::Insert(Insert {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_PRAGMA => Self::Pragma(Pragma {
                name: arg1,
                argument: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_READ => Self::Read(Read {
                table: arg1,
                column: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_SELECT => Self::Select(Select { accessor }),
            ffi::SQLITE_TRANSACTION => Self::Transaction(Transaction {
                operation: arg1,
                accessor,
            }),
            ffi::SQLITE_UPDATE => Self::Update(Update {
                table: arg1,
                column: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_ATTACH => Self::Attach(Attach {
                filename: arg1,
                accessor,
            }),
            ffi::SQLITE_DETACH => Self::Detach(Detach {
                database: arg1,
                accessor,
            }),
            ffi::SQLITE_ALTER_TABLE => Self::AlterTable(AlterTable {
                database: arg1,
                table: arg2,
                accessor,
            }),
            ffi::SQLITE_REINDEX => Self::Reindex(Reindex {
                index: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_ANALYZE => Self::Analyze(Analyze {
                table: arg1,
                database,
                accessor,
            }),
            ffi::SQLITE_CREATE_VTABLE => Self::CreateVTable(CreateVTable {
                table: arg1,
                module: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_DROP_VTABLE => Self::DropVTable(DropVTable {
                table: arg1,
                module: arg2,
                database,
                accessor,
            }),
            ffi::SQLITE_FUNCTION => Self::Function(Function {
                function: arg2,
                accessor,
            }),
            ffi::SQLITE_SAVEPOINT => Self::Savepoint(Savepoint {
                operation: arg1,
                name: arg2,
                accessor,
            }),
            ffi::SQLITE_RECURSIVE => Self::Recursive(Recursive { accessor }),
            other => Self::Unknown(Unknown {
                code: other,
                arg1,
                arg2,
                database,
                accessor,
            }),
        }
    }
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
