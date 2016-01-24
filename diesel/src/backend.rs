use query_builder::QueryBuilder;
use query_builder::pg::PgQueryBuilder;
use query_builder::debug::DebugQueryBuilder;
use types::{self, HasSqlType};

pub trait Backend where
    Self: Sized,
    Self: HasSqlType<types::SmallInt>,
    Self: HasSqlType<types::Integer>,
    Self: HasSqlType<types::BigInt>,
    Self: HasSqlType<types::Float>,
    Self: HasSqlType<types::Double>,
    Self: HasSqlType<types::Numeric>,
    Self: HasSqlType<types::VarChar>,
    Self: HasSqlType<types::Text>,
    Self: HasSqlType<types::Binary>,
    Self: HasSqlType<types::Date>,
    Self: HasSqlType<types::Time>,
    Self: HasSqlType<types::Timestamp>,
    Self: HasSqlType<()>,
{
    type QueryBuilder: QueryBuilder<Self>;
}

pub trait TypeMetadata {
    type TypeMetadata;
}

pub struct Debug;

impl Backend for Debug {
    type QueryBuilder = DebugQueryBuilder;
}

impl TypeMetadata for Debug {
    type TypeMetadata = ();
}

pub struct Pg;

#[derive(Debug, Clone, Copy, Default)]
pub struct PgTypeMetadata {
    pub oid: u32,
    pub array_oid: u32,
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
}
