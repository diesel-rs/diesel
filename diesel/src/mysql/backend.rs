//! The MySQL backend

use super::MysqlQueryBuilder;
use super::MysqlValue;
use crate::backend::*;
use crate::internal::derives::multiconnection::sql_dialect;
use crate::mysql_like::MysqlType;
use crate::mysql_like::query_fragments::MySqlLikeBatchUpdateSupport;
use crate::mysql_like::query_fragments::{
    MysqlConcatClause, MysqlOnConflictClause, MysqlRequiresOrderForWindowFunctions,
    MysqlStyleDefaultValueClause,
};
use crate::mysql_like::{MapErrorNumber, MysqlLikeBackend};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::result::DatabaseErrorKind;
use crate::sql_types::TypeMetadata;

/// The MySQL backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct Mysql;

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

    type BatchUpdateSupport = MySqlLikeBatchUpdateSupport;

    type EmptyFromClauseSyntax = sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax;
    type SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement;

    type ExistsSyntax = sql_dialect::exists_syntax::AnsiSqlExistsSyntax;
    type ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison;

    type ConcatClause = MysqlConcatClause;
    type AliasSyntax = sql_dialect::alias_syntax::AsAliasSyntax;

    type WindowFrameClauseGroupSupport =
        sql_dialect::window_frame_clause_group_support::NoGroupWindowFrameUnit;

    type WindowFrameExclusionSupport =
        sql_dialect::window_frame_exclusion_support::NoFrameFrameExclusionSupport;

    type AggregateFunctionExpressions =
        sql_dialect::aggregate_function_expressions::NoAggregateFunctionExpressions;

    type BuiltInWindowFunctionRequireOrder = MysqlRequiresOrderForWindowFunctions;
}

impl DieselReserveSpecialization for Mysql {}
impl TrustedBackend for Mysql {}

impl MysqlLikeBackend for Mysql {
    const SCHEME: &'static str = "mysql";
}

impl MapErrorNumber for Mysql {
    fn map_error_number(error_number: u32) -> crate::result::DatabaseErrorKind {
        // These values are not exposed by the C API, but are documented
        // at https://dev.mysql.com/doc/refman/8.0/en/server-error-reference.html
        // and are from the ANSI SQLSTATE standard
        match error_number {
            1062 | 1586 | 1859 => DatabaseErrorKind::UniqueViolation,
            1216 | 1217 | 1451 | 1452 | 1830 | 1834 => DatabaseErrorKind::ForeignKeyViolation,
            1792 => DatabaseErrorKind::ReadOnlyTransaction,
            1048 | 1364 => DatabaseErrorKind::NotNullViolation,
            3819 => DatabaseErrorKind::CheckViolation,
            1213 => DatabaseErrorKind::SerializationFailure,
            _ => DatabaseErrorKind::Unknown,
        }
    }
}
