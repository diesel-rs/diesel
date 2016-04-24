use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::query_builder::PgQueryBuilder;

#[derive(Debug, Clone, Copy)]
pub struct Pg;

#[derive(Debug, Clone, Copy)]
pub struct PgTypeMetadata {
    pub oid: u32,
    pub array_oid: u32,
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
    type BindCollector = RawBytesBindCollector<Pg>;
    type RawValue = [u8];
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
}

impl SupportsReturningClause for Pg {}
impl SupportsDefaultKeyword for Pg {}
