//! The MySQL backend
use byteorder::NativeEndian;

use backend::*;
use sql_types::TypeMetadata;

pub trait MysqlLikeBackend {
    type TypeMetadata;
    type QueryBuilder;
    type BindCollector;
}

impl<DB: MysqlLikeBackend> Backend for DB {
    type QueryBuilder = DB::QueryBuilder;
    type BindCollector = DB::BindCollector;
    type RawValue = [u8];
    type ByteOrder = NativeEndian;
}

impl<DB: MysqlLikeBackend> TypeMetadata for DB {
    type TypeMetadata = <DB as MysqlLikeBackend>::TypeMetadata;
    type MetadataLookup = ();
}

impl<DB: MysqlLikeBackend> SupportsDefaultKeyword for DB {}
impl<DB: MysqlLikeBackend> UsesAnsiSavepointSyntax for DB {}
