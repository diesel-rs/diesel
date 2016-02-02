use backend::*;
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
    type RawValue = SqliteValue;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
}
