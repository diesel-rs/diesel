use query_builder::QueryBuilder;
use query_builder::debug::DebugQueryBuilder;
use types::{self, HasSqlType};

pub trait Backend where
    Self: Sized,
    Self: HasSqlType<types::SmallInt>,
    Self: HasSqlType<types::Integer>,
    Self: HasSqlType<types::BigInt>,
    Self: HasSqlType<types::Float>,
    Self: HasSqlType<types::Double>,
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
pub trait SupportsDefaultKeyword {}
pub trait SupportsNowFunction {}

pub struct Debug;

impl Backend for Debug {
    type QueryBuilder = DebugQueryBuilder;
    type RawValue = ();
}

impl TypeMetadata for Debug {
    type TypeMetadata = ();
}

impl SupportsReturningClause for Debug {}
impl SupportsDefaultKeyword for Debug {}
impl SupportsNowFunction for Debug {}
