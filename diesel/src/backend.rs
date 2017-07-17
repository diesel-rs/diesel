use byteorder::{ByteOrder, NativeEndian};

use query_builder::QueryBuilder;
use query_builder::bind_collector::{BindCollector, RawBytesBindCollector};
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
    type BindCollector: BindCollector<Self>;
    type RawValue: ?Sized;
    type ByteOrder: ByteOrder;
}

pub trait TypeMetadata {
    type TypeMetadata;
    type MetadataLookup;
}

pub trait SupportsReturningClause {}
pub trait SupportsDefaultKeyword {}
pub trait UsesAnsiSavepointSyntax {}

#[derive(Debug, Copy, Clone)]
pub struct Debug;

impl Backend for Debug {
    type QueryBuilder = DebugQueryBuilder;
    type BindCollector = RawBytesBindCollector<Self>;
    type RawValue = ();
    type ByteOrder = NativeEndian;
}

impl TypeMetadata for Debug {
    type TypeMetadata = ();
    type MetadataLookup = ();
}

impl SupportsReturningClause for Debug {}
impl SupportsDefaultKeyword for Debug {}
impl UsesAnsiSavepointSyntax for Debug {}
