use byteorder::NetworkEndian;

use backend::*;
use prelude::Queryable;
use query_builder::bind_collector::RawBytesBindCollector;
use super::PgMetadataLookup;
use super::query_builder::PgQueryBuilder;
use types::Oid;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct PgTypeMetadata {
    pub oid: u32,
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
