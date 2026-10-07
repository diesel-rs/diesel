use crate::backend::sql_dialect::returning_clause::SupportsReturningClause;
use crate::backend::{
    Backend, DieselReserveSpecialization, SqlDialect, TrustedBackend, sql_dialect,
};
use crate::mariadb::{MariadbQueryBuilder, MariadbValue};
use crate::mysql_like::query_fragments::MySqlLikeBatchUpdateSupport;
use crate::mysql_like::{MapErrorNumber, MysqlLikeBackend};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::result::DatabaseErrorKind;
use crate::sql_types::TypeMetadata;

/// The MariaDB backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct Mariadb;

/// Represents possible types, that can be transmitted as via the
/// Mysql wire protocol
pub type MariadbType = crate::mysql_like::MysqlType;

impl Backend for Mariadb {
    type QueryBuilder = MariadbQueryBuilder;
    type RawValue<'a> = MariadbValue<'a>;
    type BindCollector<'a> = RawBytesBindCollector<Self>;
}

impl TypeMetadata for Mariadb {
    type TypeMetadata = MariadbType;

    type MetadataLookup = ();
}

impl SqlDialect for Mariadb {
    type ReturningClause = MariadbReturningClause;

    type OnConflictClause = MariadbOnConflictClause;

    type InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword;
    type BatchInsertSupport = sql_dialect::batch_insert_support::PostgresLikeBatchInsertSupport;
    type DefaultValueClauseForInsert = MariadbStyleDefaultValueClause;

    type BatchUpdateSupport = MySqlLikeBatchUpdateSupport;

    type EmptyFromClauseSyntax = sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax;
    type SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement;

    type ExistsSyntax = sql_dialect::exists_syntax::AnsiSqlExistsSyntax;
    type ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison;

    type ConcatClause = MariadbConcatClause;
    type AliasSyntax = sql_dialect::alias_syntax::AsAliasSyntax;

    type WindowFrameClauseGroupSupport =
        sql_dialect::window_frame_clause_group_support::NoGroupWindowFrameUnit;

    type WindowFrameExclusionSupport =
        sql_dialect::window_frame_exclusion_support::NoFrameFrameExclusionSupport;

    type AggregateFunctionExpressions =
        sql_dialect::aggregate_function_expressions::NoAggregateFunctionExpressions;

    type BuiltInWindowFunctionRequireOrder = MariadbRequiresOrderForWindowFunctions;
}

impl DieselReserveSpecialization for Mariadb {}
impl TrustedBackend for Mariadb {}

pub(crate) type MariadbOnConflictClause = crate::mysql_like::query_fragments::MysqlOnConflictClause;
pub(crate) type MariadbStyleDefaultValueClause =
    crate::mysql_like::query_fragments::MysqlStyleDefaultValueClause;
pub(crate) type MariadbConcatClause = crate::mysql_like::query_fragments::MysqlConcatClause;
pub(crate) type MariadbRequiresOrderForWindowFunctions =
    crate::mysql_like::query_fragments::MysqlRequiresOrderForWindowFunctions;

#[derive(Debug, Clone, Copy)]
pub struct MariadbReturningClause;

impl SupportsReturningClause for MariadbReturningClause {}

impl MysqlLikeBackend for Mariadb {
    const SCHEME: &'static str = "mariadb";
}

impl MapErrorNumber for Mariadb {
    fn map_error_number(error_number: u32) -> crate::result::DatabaseErrorKind {
        // These values are not exposed by the C API, but are documented
        // at https://mariadb.com/docs/server/reference/error-codes/mariadb-error-code-reference
        match error_number {
            1022 | 1062 | 1586 | 1859 => DatabaseErrorKind::UniqueViolation,
            1216 | 1217 | 1451 | 1557 | 1452 | 1830 | 1834 => {
                DatabaseErrorKind::ForeignKeyViolation
            }
            1792 => DatabaseErrorKind::ReadOnlyTransaction,
            1048 | 1364 => DatabaseErrorKind::NotNullViolation,
            4025 => DatabaseErrorKind::CheckViolation,
            1213 => DatabaseErrorKind::SerializationFailure,
            _ => DatabaseErrorKind::Unknown,
        }
    }
}
