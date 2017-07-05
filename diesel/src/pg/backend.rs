use byteorder::NetworkEndian;

use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::query_builder::PgQueryBuilder;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum IsArray {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum PgTypeMetadata {
    Static {
        oid: u32,
        array_oid: u32,
    },
    Dynamic {
        schema: &'static str,
        typename: &'static str,
        as_array: IsArray,
    },
}

#[allow(missing_debug_implementations)]
pub struct PgMetadataLookup {
    c: super::connection::PgConnection,
}

impl PgMetadataLookup {
    pub(super) fn new(conn: &super::connection::PgConnection) -> &Self {
        unsafe{ ::std::mem::transmute(conn) }
    }
}

impl MetadataLookup<PgTypeMetadata> for PgMetadataLookup {
    type MetadataIdentifier = u32;

    fn lookup(&self, t: &PgTypeMetadata) -> ::result::QueryResult<u32> {
        self.c.lookup(t)
    }
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
    type BindCollector = RawBytesBindCollector<Pg>;
    type RawValue = [u8];
    type ByteOrder = NetworkEndian;
    type MetadataLookup = PgMetadataLookup;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
}

impl SupportsReturningClause for Pg {}
impl SupportsDefaultKeyword for Pg {}
impl UsesAnsiSavepointSyntax for Pg {}
