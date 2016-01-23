use connection::sqlite::SqliteValue;
use query_builder::QueryBuilder;
use query_builder::debug::DebugQueryBuilder;
use query_builder::pg::PgQueryBuilder;
use query_builder::sqlite::SqliteQueryBuilder;
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
{
    type QueryBuilder: QueryBuilder<Self>;
    type RawValue: ?Sized;
}

pub trait TypeMetadata {
    type TypeMetadata;
}

pub trait SupportsReturningClause {}

pub struct Debug;

impl Backend for Debug {
    type QueryBuilder = DebugQueryBuilder;
    type RawValue = ();
}

impl TypeMetadata for Debug {
    type TypeMetadata = ();
}

impl SupportsReturningClause for Debug {}

pub struct Pg;

#[derive(Debug, Clone, Copy)]
pub struct PgTypeMetadata {
    pub oid: u32,
    pub array_oid: u32,
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
    type RawValue = [u8];
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
}

impl SupportsReturningClause for Pg {}

pub struct Sqlite;

pub enum SqliteType {
    Binary,
    Text,
    Float,
    Double,
    SmallInt,
    Integer,
    Long,
}

impl Backend for Sqlite {
    type QueryBuilder = SqliteQueryBuilder;
    type RawValue = SqliteValue;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
}
