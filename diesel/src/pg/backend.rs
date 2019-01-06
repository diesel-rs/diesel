//! The PostgreSQL backend

use byteorder::NetworkEndian;

use super::query_builder::PgQueryBuilder;
use super::PgMetadataLookup;
use crate::backend::*;
use crate::prelude::Queryable;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::sql_types::{Oid, TypeMetadata};

/// The PostgreSQL backend
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

/// The [OIDs] for a SQL type
///
/// [OIDs]: https://www.postgresql.org/docs/current/static/datatype-oid.html
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct PgTypeMetadata {
    /// The [OID] of `T`
    ///
    /// [OID]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    pub oid: u32,
    /// The [OID] of `T[]`
    ///
    /// [OID]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    pub array_oid: u32,
}

impl Queryable<(Oid, Oid), Pg> for PgTypeMetadata {
    type Row = (u32, u32);

    fn build((oid, array_oid): Self::Row) -> Self {
        PgTypeMetadata { oid, array_oid }
    }
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
    type BindCollector = RawBytesBindCollector<Pg>;
    type RawValue = [u8];
    type ByteOrder = NetworkEndian;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
    type MetadataLookup = PgMetadataLookup;
}

impl SupportsReturningClause for Pg {}
impl SupportsDefaultKeyword for Pg {}
impl UsesAnsiSavepointSyntax for Pg {}
