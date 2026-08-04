use crate::{
    backend::{Backend, DieselReserveSpecialization, SqlDialect, TrustedBackend, sql_dialect},
    mariadb::{MariadbQueryBuilder, MariadbValue},
    mysql_like::MysqlLikeBackend,
    query_builder::bind_collector::RawBytesBindCollector,
    sql_types::TypeMetadata,
};

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
    type ReturningClause = sql_dialect::returning_clause::DoesNotSupportReturningClause;

    type OnConflictClause = MariadbOnConflictClause;

    type InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword;
    type BatchInsertSupport = sql_dialect::batch_insert_support::PostgresLikeBatchInsertSupport;
    type DefaultValueClauseForInsert = MariadbStyleDefaultValueClause;

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

impl MysqlLikeBackend for Mariadb {
    const SCHEME: &'static str = "mariadb";
}
