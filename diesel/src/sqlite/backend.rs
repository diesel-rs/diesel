use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::connection::SqliteValue;
use super::query_builder::SqliteQueryBuilder;

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
    type BindCollector = RawBytesBindCollector<Sqlite>;
    type RawValue = SqliteValue;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
}
