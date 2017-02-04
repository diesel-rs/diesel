use byteorder::NativeEndian;

use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::connection::SqliteValue;
use super::query_builder::SqliteQueryBuilder;

#[derive(Debug, Copy, Clone)]
pub struct Sqlite;

#[allow(missing_debug_implementations, missing_copy_implementations)]
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
    type BindCollector = RawBytesBindCollector<Sqlite>;
    type RawValue = SqliteValue;
    type ByteOrder = NativeEndian;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
}

impl UsesAnsiSavepointSyntax for Sqlite {}
