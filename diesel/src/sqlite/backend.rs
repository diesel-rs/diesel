//! The SQLite backend

use super::connection::{SqliteBindCollector, SqliteValue};
use super::query_builder::SqliteQueryBuilder;
use crate::backend::*;
use crate::sql_types::TypeMetadata;

/// The SQLite backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
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
}

impl<'a> HasBindCollector<'a> for Sqlite {
    type BindCollector = SqliteBindCollector<'a>;
}

impl<'a> HasRawValue<'a> for Sqlite {
    type RawValue = SqliteValue<'a, 'a, 'a>;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
    type MetadataLookup = ();
}

impl SqlDialect for Sqlite {
    type ReturningClause = sql_dialect::returning_clause::DoesNotSupportReturningClause;

    type OnConflictClause = SqliteOnConflictClaues;

    type InsertWithDefaultKeyword =
        sql_dialect::default_keyword_for_insert::DoesNotSupportDefaultKeyword;
    type BatchInsertSupport = SqliteBatchInsert;
    type DefaultValueClauseForInsert = sql_dialect::default_value_clause::AnsiDefaultValueClause;

    type EmptyFromClauseSyntax = sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax;
    type ExistsSyntax = sql_dialect::exists_syntax::AnsiSqlExistsSyntax;
    type ArrayComparision = sql_dialect::array_comparision::AnsiSqlArrayComparison;
}

#[derive(Debug, Copy, Clone)]
pub struct SqliteOnConflictClaues;

impl sql_dialect::on_conflict_clause::SupportsOnConflictClause for SqliteOnConflictClaues {}

#[derive(Debug, Copy, Clone)]
pub struct SqliteBatchInsert;
