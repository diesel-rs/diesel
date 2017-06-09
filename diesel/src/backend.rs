use byteorder::{ByteOrder, NativeEndian};

use query_builder::QueryBuilder;
use query_builder::bind_collector::{BindCollector, RawBytesBindCollector};
use query_builder::debug::DebugQueryBuilder;
use types::{self, HasSqlType};
use result::QueryResult;

pub trait Backend where
    Self: Sized,
    Self: TypeMetadata,
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
    type MetadataLookup: MetadataLookup<Self::TypeMetadata>;
}

pub trait TypeMetadata {
    type TypeMetadata;
}

pub trait MetadataLookup<T> {
    type MetadataIdentifier;
    fn lookup(&self, t: &T) ->  QueryResult<Self::MetadataIdentifier>;
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
    type MetadataLookup = ();
}

impl TypeMetadata for Debug {
    type TypeMetadata = ();
}

impl MetadataLookup<()> for () {
    type MetadataIdentifier = ();
    fn lookup(&self, _: &()) -> QueryResult<()> {
        Ok(())
    }
}

impl SupportsReturningClause for Debug {}
impl SupportsDefaultKeyword for Debug {}
impl UsesAnsiSavepointSyntax for Debug {}
