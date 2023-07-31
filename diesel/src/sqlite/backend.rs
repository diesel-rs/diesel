//! The SQLite backend

use super::connection::{SqliteBindCollector, SqliteValue};
use super::query_builder::SqliteQueryBuilder;
use crate::backend::*;
use crate::sql_types::TypeMetadata;

/// The SQLite backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct Sqlite;

/// Determines how a bind parameter is given to SQLite
///
/// Diesel deals with bind parameters after serialization as opaque blobs of
/// bytes. However, SQLite instead has several functions where it expects the
/// relevant C types.
///
/// The variants of this struct determine what bytes are expected from
/// `ToSql` impls.
#[allow(missing_debug_implementations)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SqliteType {
    /// Bind using `sqlite3_bind_blob`
    Binary,
    /// Bind using `sqlite3_bind_text`
    Text,
    /// `bytes` should contain an `f32`
    Float,
    /// `bytes` should contain an `f64`
    Double,
    /// `bytes` should contain an `i16`
    SmallInt,
    /// `bytes` should contain an `i32`
    Integer,
    /// `bytes` should contain an `i64`
    Long,
}

impl Backend for Sqlite {
    type QueryBuilder = SqliteQueryBuilder;
    type RawValue<'a> = SqliteValue<'a, 'a, 'a>;
    type BindCollector<'a> = SqliteBindCollector<'a>;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
    type MetadataLookup = ();
}

impl SqlDialect for Sqlite {
    #[cfg(not(feature = "returning_clauses_for_sqlite_3_35"))]
    type ReturningClause = sql_dialect::returning_clause::DoesNotSupportReturningClause;
    #[cfg(feature = "returning_clauses_for_sqlite_3_35")]
    type ReturningClause = SqliteReturningClause;

    type OnConflictClause = SqliteOnConflictClause;

    type InsertWithDefaultKeyword =
        sql_dialect::default_keyword_for_insert::DoesNotSupportDefaultKeyword;
    type BatchInsertSupport = SqliteBatchInsert;
    type ConcatClause = sql_dialect::concat_clause::ConcatWithPipesClause;
    type DefaultValueClauseForInsert = sql_dialect::default_value_clause::AnsiDefaultValueClause;

    type EmptyFromClauseSyntax = sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax;
    type SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement;

    type ExistsSyntax = sql_dialect::exists_syntax::AnsiSqlExistsSyntax;
    type ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison;
    type AliasSyntax = sql_dialect::alias_syntax::AsAliasSyntax;
}

impl DieselReserveSpecialization for Sqlite {}
impl TrustedBackend for Sqlite {}

#[derive(Debug, Copy, Clone)]
pub struct SqliteOnConflictClause;

impl sql_dialect::on_conflict_clause::SupportsOnConflictClause for SqliteOnConflictClause {}
impl sql_dialect::on_conflict_clause::PgLikeOnConflictClause for SqliteOnConflictClause {}

#[derive(Debug, Copy, Clone)]
pub struct SqliteBatchInsert;

#[derive(Debug, Copy, Clone)]
pub struct SqliteReturningClause;

impl sql_dialect::returning_clause::SupportsReturningClause for SqliteReturningClause {}
