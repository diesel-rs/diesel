use byteorder::NativeEndian;

use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::query_builder::MysqlQueryBuilder;

#[derive(Debug, Copy, Clone)]
pub struct Mysql;

#[allow(missing_debug_implementations, missing_copy_implementations)]
/// Represents the possible forms a bind parameter can be transmitted as.
/// Each variant represents one of the forms documented at
/// https://dev.mysql.com/doc/refman/5.7/en/c-api-prepared-statement-type-codes.html
///
/// The null variant is omitted, as we will never prepare a statement in which
/// one of the bind parameters can always be NULL
pub enum MysqlType {
    Tiny,
    Short,
    Long,
    LongLong,
    Float,
    Double,
    Time,
    Date,
    DateTime,
    Timestamp,
    String,
    Blob,
}

impl Backend for Mysql {
    type QueryBuilder = MysqlQueryBuilder;
    type BindCollector = RawBytesBindCollector<Mysql>;
    type RawValue = [u8];
    type ByteOrder = NativeEndian;
}

impl TypeMetadata for Mysql {
    type TypeMetadata = MysqlType;
}

impl SupportsDefaultKeyword for Mysql {}
impl UsesAnsiSavepointSyntax for Mysql {}
