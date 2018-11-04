//! The MySQL backend
use byteorder::NativeEndian;

use mysql::bind_collector::MysqlBindCollector;
use mysql::query_builder::MysqlQueryBuilder;
use backend::*;
use sql_types::TypeMetadata;

pub trait MysqlLikeBackend {
    type TypeMetadata;
}

impl<DB: MysqlLikeBackend> Backend for DB {
    type QueryBuilder = MysqlQueryBuilder;
    type BindCollector = MysqlBindCollector;
    type RawValue = [u8];
    type ByteOrder = NativeEndian;
}

impl<DB: MysqlLikeBackend> TypeMetadata for DB {
    type TypeMetadata = <DB as MysqlLikeBackend>::TypeMetadata;
    type MetadataLookup = ();
}

impl<DB: MysqlLikeBackend> SupportsDefaultKeyword for DB {}
impl<DB: MysqlLikeBackend> UsesAnsiSavepointSyntax for DB {}
