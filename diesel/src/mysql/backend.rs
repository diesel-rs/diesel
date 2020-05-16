//! The MySQL backend

use byteorder::NativeEndian;

use super::query_builder::MysqlQueryBuilder;
use super::MysqlValue;
use crate::backend::*;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::sql_types::TypeMetadata;

/// The MySQL backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Mysql;

#[allow(missing_debug_implementations)]
/// Represents the possible types, that can be transmitted as via the
/// Mysql wire protocol
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MysqlType {
    ///
    Tiny,
    /// Sets
    UnsignedTiny,
    /// Sets `buffer_type` to `MYSQL_TYPE_SHORT`
    Short,
    ///
    UnsignedShort,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONG`
    Long,
    ///
    UnsignedLong,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONGLONG`
    LongLong,
    ///
    UnsignedLongLong,
    /// Sets `buffer_type` to `MYSQL_TYPE_FLOAT`
    Float,
    /// Sets `buffer_type` to `MYSQL_TYPE_DOUBLE`
    Double,
    /// Sets `buffer_type` to `MYSQL_TYPE_NEWDECIMAL`
    Numeric,
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
    /// Sets `buffer_type` to `MYSQL_TYPE_BIT`
    Bit,
    ///
    Set,
    ///
    Enum,
    ///
    Json,
}

impl Backend for Mysql {
    type QueryBuilder = MysqlQueryBuilder;
    type BindCollector = RawBytesBindCollector<Self>;
    type ByteOrder = NativeEndian;
}

impl<'a> HasRawValue<'a> for Mysql {
    type RawValue = MysqlValue<'a>;
}

impl TypeMetadata for Mysql {
    type TypeMetadata = MysqlType;
    type MetadataLookup = ();
}

impl SupportsDefaultKeyword for Mysql {}
impl UsesAnsiSavepointSyntax for Mysql {}
