//! The MySQL backend
use byteorder::NativeEndian;

use backend::*;
use mysql::bind_collector::MysqlBindCollector;
use mysql::query_builder::MysqlQueryBuilder;
use mysql::MysqlType;
use sql_types::TypeMetadata;

pub trait MysqlLikeBackend: Backend + for<'a> HasRawValue<'a, RawValue = &'a [u8]> {}

impl<DB: MysqlLikeBackend> Backend for DB {
    type QueryBuilder = MysqlQueryBuilder;
    type BindCollector = MysqlBindCollector;
    type ByteOrder = NativeEndian;
}

impl<'a, DB: MysqlLikeBackend> HasRawValue<'a> for DB {
    type RawValue = &'a [u8];
}

impl<DB: MysqlLikeBackend> TypeMetadata for DB {
    type TypeMetadata = MysqlType;
    type MetadataLookup = ();
}

impl<DB: MysqlLikeBackend> SupportsDefaultKeyword for DB {}
impl<DB: MysqlLikeBackend> UsesAnsiSavepointSyntax for DB {}
