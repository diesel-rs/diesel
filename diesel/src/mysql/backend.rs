//! The MySQL backend

use byteorder::NativeEndian;

use super::query_builder::MysqlQueryBuilder;
use super::MysqlValue;
use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use sql_types::TypeMetadata;

/// The MySQL backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Mysql;

/// The full type metadata for MySQL
///
/// This includes the type of the value, and whether it is signed.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct MysqlTypeMetadata {
    /// The underlying data type
    ///
    /// Affects the `buffer_type` sent to libmysqlclient
    pub data_type: MysqlType,

    /// Is this type signed?
    ///
    /// Affects the `is_unsigned` flag sent to libmysqlclient
    pub is_unsigned: bool,
}

#[allow(missing_debug_implementations)]
/// Represents the possible forms a bind parameter can be transmitted as.
/// Each variant represents one of the forms documented at
/// <https://dev.mysql.com/doc/refman/5.7/en/c-api-prepared-statement-type-codes.html>
///
/// The null variant is omitted, as we will never prepare a statement in which
/// one of the bind parameters can always be NULL
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MysqlType {
    /// Sets `buffer_type` to `MYSQL_TYPE_TINY`
    Tiny,
    /// Sets `buffer_type` to `MYSQL_TYPE_SHORT`
    Short,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONG`
    Long,
    /// Sets `buffer_type` to `MYSQL_TYPE_LONGLONG`
    LongLong,
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

impl Backend for Mysql {
    type QueryBuilder = MysqlQueryBuilder;
    type BindCollector = RawBytesBindCollector<Self>;
    type ByteOrder = NativeEndian;
}

impl<'a> HasRawValue<'a> for Mysql {
    type RawValue = MysqlValue<'a>;
}

impl TypeMetadata for Mysql {
    type TypeMetadata = MysqlTypeMetadata;
    type MetadataLookup = ();
}

impl SupportsDefaultKeyword for Mysql {}
impl UsesAnsiSavepointSyntax for Mysql {}
