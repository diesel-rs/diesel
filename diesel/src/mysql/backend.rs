use byteorder::NativeEndian;

use backend::*;
use query_builder::bind_collector::RawBytesBindCollector;
use super::query_builder::MysqlQueryBuilder;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Mysql;

#[allow(missing_debug_implementations)]
/// Represents the possible forms a bind parameter can be transmitted as.
/// Each variant represents one of the forms documented at
/// https://dev.mysql.com/doc/refman/5.7/en/c-api-prepared-statement-type-codes.html
///
/// The null variant is omitted, as we will never prepare a statement in which
/// one of the bind parameters can always be NULL
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
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

// FIXME: Move this out of this module
use types::HasSqlType;

impl HasSqlType<::types::Date> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Date
    }
}

impl HasSqlType<::types::Time> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Time
    }
}

impl HasSqlType<::types::Timestamp> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Timestamp
    }
}

use types::{ToSql, IsNull, FromSql};
use std::error::Error as StdError;
use std::io::Write;

impl ToSql<::types::Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
        let int_value = if *self {
            1
        } else {
            0
        };
        <i32 as ToSql<::types::Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<::types::Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}
