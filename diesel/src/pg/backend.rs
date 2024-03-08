//! The PostgreSQL backend

use super::query_builder::PgQueryBuilder;
use super::{PgMetadataLookup, PgValue};
use crate::backend::*;
use crate::deserialize::Queryable;
use crate::expression::operators::LikeIsAllowedForType;
use crate::pg::metadata_lookup::PgMetadataCacheKey;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::sql_types::TypeMetadata;

/// The PostgreSQL backend
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct Pg;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Queryable)]
pub struct InnerPgTypeMetadata {
    pub(in crate::pg) oid: u32,
    pub(in crate::pg) array_oid: u32,
}

impl From<(u32, u32)> for InnerPgTypeMetadata {
    fn from((oid, array_oid): (u32, u32)) -> Self {
        Self { oid, array_oid }
    }
}

/// This error indicates that a type lookup for a custom
/// postgres type failed
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
#[allow(unreachable_pub)]
pub struct FailedToLookupTypeError(Box<PgMetadataCacheKey<'static>>);

impl FailedToLookupTypeError {
    /// Construct a new instance of this error type
    /// containing information about which type lookup failed
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn new(cache_key: PgMetadataCacheKey<'static>) -> Self {
        Self::new_internal(cache_key)
    }

    pub(in crate::pg) fn new_internal(cache_key: PgMetadataCacheKey<'static>) -> Self {
        Self(Box::new(cache_key))
    }
}

impl std::error::Error for FailedToLookupTypeError {}

impl std::fmt::Display for FailedToLookupTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = self.0.schema.as_ref() {
            write!(
                f,
                "Failed to find a type oid for `{}`.`{}`",
                schema, self.0.type_name
            )
        } else {
            write!(f, "Failed to find a type oid for `{}`", self.0.type_name)
        }
    }
}

/// The [OIDs] for a SQL type
///
/// [OIDs]: https://www.postgresql.org/docs/current/static/datatype-oid.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub struct PgTypeMetadata(pub(in crate::pg) Result<InnerPgTypeMetadata, FailedToLookupTypeError>);

impl PgTypeMetadata {
    /// Create a new instance of this type based on known constant [OIDs].
    ///
    /// Please refer to [PgMetadataLookup] for a way to query [OIDs]
    /// of custom types at run time
    ///
    /// [OIDs]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    /// [PgMetadataLookup]: struct.PgMetadataLookup.html
    pub fn new(type_oid: u32, array_oid: u32) -> Self {
        Self(Ok(InnerPgTypeMetadata {
            oid: type_oid,
            array_oid,
        }))
    }

    /// Create a new instance of this type based on dynamically lookup information
    ///
    /// This function is useful for third party crates that may implement a custom
    /// postgres connection type and want to bring their own lookup mechanism.
    ///
    /// Otherwise refer to [PgMetadataLookup] for a way to automatically
    /// implement the corresponding lookup functionality
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn from_result(r: Result<(u32, u32), FailedToLookupTypeError>) -> Self {
        Self(r.map(|(oid, array_oid)| InnerPgTypeMetadata { oid, array_oid }))
    }

    /// The [OID] of `T`
    ///
    /// [OID]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    pub fn oid(&self) -> Result<u32, impl std::error::Error + Send + Sync> {
        self.0.as_ref().map(|i| i.oid).map_err(Clone::clone)
    }

    /// The [OID] of `T[]`
    ///
    /// [OID]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    pub fn array_oid(&self) -> Result<u32, impl std::error::Error + Send + Sync> {
        self.0.as_ref().map(|i| i.array_oid).map_err(Clone::clone)
    }
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
    type RawValue<'a> = PgValue<'a>;
    type BindCollector<'a> = RawBytesBindCollector<Pg>;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
    type MetadataLookup = dyn PgMetadataLookup;
}

impl SqlDialect for Pg {
    type ReturningClause = sql_dialect::returning_clause::PgLikeReturningClause;

    type OnConflictClause = PgOnConflictClause;

    type InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword;
    type BatchInsertSupport = sql_dialect::batch_insert_support::PostgresLikeBatchInsertSupport;
    type ConcatClause = sql_dialect::concat_clause::ConcatWithPipesClause;

    type DefaultValueClauseForInsert = sql_dialect::default_value_clause::AnsiDefaultValueClause;

    type EmptyFromClauseSyntax = sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax;
    type SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement;

    type ExistsSyntax = sql_dialect::exists_syntax::AnsiSqlExistsSyntax;
    type ArrayComparison = PgStyleArrayComparison;
    type AliasSyntax = sql_dialect::alias_syntax::AsAliasSyntax;
}

impl DieselReserveSpecialization for Pg {}
impl TrustedBackend for Pg {}

#[derive(Debug, Copy, Clone)]
pub struct PgOnConflictClause;

impl sql_dialect::on_conflict_clause::SupportsOnConflictClause for PgOnConflictClause {}
impl sql_dialect::on_conflict_clause::SupportsOnConflictClauseWhere for PgOnConflictClause {}
impl sql_dialect::on_conflict_clause::PgLikeOnConflictClause for PgOnConflictClause {}

#[derive(Debug, Copy, Clone)]
pub struct PgStyleArrayComparison;

impl LikeIsAllowedForType<crate::sql_types::Binary> for Pg {}
