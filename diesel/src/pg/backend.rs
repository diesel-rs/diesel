use byteorder::NetworkEndian;

use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::PgMetadataLookup;
use super::query_builder::PgQueryBuilder;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum PgTypeMetadata {
    Stable {
        oid: u32,
        array_oid: u32,
    },
    Lookup(&'static str),
    ArrayLookup(&'static str),
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
