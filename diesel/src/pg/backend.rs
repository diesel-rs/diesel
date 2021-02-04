//! The PostgreSQL backend

use byteorder::NetworkEndian;

use super::query_builder::PgQueryBuilder;
use super::{PgMetadataLookup, PgValue};
use crate::backend::*;
use crate::deserialize::Queryable;
use crate::pg::metadata_lookup::PgMetadataCacheKey;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::sql_types::TypeMetadata;

/// The PostgreSQL backend
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Queryable)]
pub(in crate::pg) struct InnerPgTypeMetadata {
    pub oid: u32,
    pub array_oid: u32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(in crate::pg) struct FailedToLookupTypeError(Box<PgMetadataCacheKey<'static>>);

impl FailedToLookupTypeError {
    pub(crate) fn new(cache_key: PgMetadataCacheKey<'static>) -> Self {
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
    type BindCollector = RawBytesBindCollector<Pg>;
    type ByteOrder = NetworkEndian;
}

impl<'a> HasRawValue<'a> for Pg {
    type RawValue = PgValue<'a>;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
    type MetadataLookup = dyn PgMetadataLookup;
}

impl SupportsReturningClause for Pg {}
impl SupportsOnConflictClause for Pg {}
impl SupportsOnConflictTargetDecorations for Pg {}
impl SupportsDefaultKeyword for Pg {}
impl UsesAnsiSavepointSyntax for Pg {}
