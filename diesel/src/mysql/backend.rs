//! The MySQL backend

use byteorder::NativeEndian;

use super::query_builder::MysqlQueryBuilder;
use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use sql_types::TypeMetadata;

/// The MySQL backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Mysql;

#[allow(missing_debug_implementations)]
/// Represents the possible forms a bind parameter can be transmitted as.
/// Each variant represents one of the forms documented at
/// <https://dev.mysql.com/doc/refman/5.7/en/c-api-prepared-statement-type-codes.html>
///
/// The null variant is omitted, as we will never prepare a statement in which
/// one of the bind parameters can always be NULL
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum MysqlType {
    /// Sets `buffer_type` to `MYSQL_TYPE_TINY`
    Tiny,
    /// Sets `buffer_type` to `MYSQL_TYPE_TINY` and `is_unsigned` to `true`
    UnsignedTiny,
    /// Sets `buffer_type` to `MYSQL_TYPE_SHORT`
    Short,
    /// Sets `buffer_type` to `MYSQL_TYPE_SHORT` and `is_unsigned` to `true`
    UnsignedShort,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONG`
    Long,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONG` and `is_unsigned` to `true`
    UnsignedLong,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONGLONG`
    LongLong,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONGLONG` and `is_unsigned` to `true`
    UnsignedLongLong,
    /// Sets `buffer_type` to `MYSQL_TYPE_FLOAT`
    Float,
    /// Sets `buffer_type` to `MYSQL_TYPE_DOUBLE`
    Double,
    /// Sets `buffer_type` to `MYSQL_TYPE_TIME`
    Time,
    /// Sets `buffer_type` to `MYSQL_TYPE_DATE`
    Date,
    /// Sets `buffer_type` to `MYSQL_TYPE_DATETIME`
    DateTime,
    /// Sets `buffer_type` to `MYSQL_TYPE_TIMESTAMP`
    Timestamp,
    /// Sets `buffer_type` to `MYSQL_TYPE_STRING`
    String,
    /// Sets `buffer_type` to `MYSQL_TYPE_BLOB`
    Blob,
}

impl MysqlType {
    /// Is this an unsigned type?
    pub fn is_unsigned(&self) -> bool {
        match *self {
            MysqlType::UnsignedTiny
            | MysqlType::UnsignedShort
            | MysqlType::UnsignedLong
            | MysqlType::UnsignedLongLong => true,
            _ => false,
        }
    }
}

impl Backend for Mysql {
    type QueryBuilder = MysqlQueryBuilder;
    type BindCollector = RawBytesBindCollector<Mysql>;
    type RawValue = [u8];
    type ByteOrder = NativeEndian;
}

impl TypeMetadata for Mysql {
    type TypeMetadata = MysqlType;
    type MetadataLookup = ();
}

impl SupportsDefaultKeyword for Mysql {}
impl UsesAnsiSavepointSyntax for Mysql {}
