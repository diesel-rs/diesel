//! The MySQL backend

use super::query_builder::MysqlQueryBuilder;
use super::MysqlValue;
use crate::backend::sql_dialect::on_conflict_clause::SupportsOnConflictClause;
use crate::backend::*;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::sql_types::TypeMetadata;

/// The MySQL backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct Mysql;

#[allow(missing_debug_implementations)]
/// Represents possible types, that can be transmitted as via the
/// Mysql wire protocol
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MysqlType {
    /// A 8 bit signed integer
    Tiny,
    /// A 8 bit unsigned integer
    UnsignedTiny,
    /// A 16 bit signed integer
    Short,
    /// A 16 bit unsigned integer
    UnsignedShort,
    /// A 32 bit signed integer
    Long,
    /// A 32 bit unsigned integer
    UnsignedLong,
    /// A 64 bit signed integer
    LongLong,
    /// A 64 bit unsigned integer
    UnsignedLongLong,
    /// A 32 bit floating point number
    Float,
    /// A 64 bit floating point number
    Double,
    /// A fixed point decimal value
    Numeric,
    /// A datatype to store a time value
    Time,
    /// A datatype to store a date value
    Date,
    /// A datatype containing timestamp values ranging from
    /// '1000-01-01 00:00:00' to '9999-12-31 23:59:59'.
    DateTime,
    /// A datatype containing timestamp values ranging from
    /// 1970-01-01 00:00:01' UTC to '2038-01-19 03:14:07' UTC.
    Timestamp,
    /// A datatype for string values
    String,
    /// A datatype containing binary large objects
    Blob,
    /// A value containing a set of bit's
    Bit,
    /// A user defined set type
    Set,
    /// A user defined enum type
    Enum,
}

impl Backend for Mysql {
    type QueryBuilder = MysqlQueryBuilder;
    type RawValue<'a> = MysqlValue<'a>;
    type BindCollector<'a> = RawBytesBindCollector<Self>;
}

impl TypeMetadata for Mysql {
    type TypeMetadata = MysqlType;
    type MetadataLookup = ();
}

impl SqlDialect for Mysql {
    type ReturningClause = sql_dialect::returning_clause::DoesNotSupportReturningClause;

    type OnConflictClause = MysqlOnConflictClause;

    type InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword;
    type BatchInsertSupport = sql_dialect::batch_insert_support::PostgresLikeBatchInsertSupport;
    type DefaultValueClauseForInsert = MysqlStyleDefaultValueClause;

    type EmptyFromClauseSyntax = sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax;
    type SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement;

    type ExistsSyntax = sql_dialect::exists_syntax::AnsiSqlExistsSyntax;
    type ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison;

    type ConcatClause = MysqlConcatClause;
    type AliasSyntax = sql_dialect::alias_syntax::AsAliasSyntax;
}

impl DieselReserveSpecialization for Mysql {}
impl TrustedBackend for Mysql {}

#[derive(Debug, Clone, Copy)]
pub struct MysqlStyleDefaultValueClause;

#[derive(Debug, Clone, Copy)]
pub struct MysqlConcatClause;

#[derive(Debug, Clone, Copy)]
pub struct MysqlOnConflictClause;

impl SupportsOnConflictClause for MysqlOnConflictClause {}
