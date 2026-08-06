//! Provides shared types and functions related to working with MySQL and MariaDB

#[cfg(any(feature = "mysql", feature = "mariadb"))]
mod connection;
mod types;

pub(crate) mod query_builder;
mod value;

use core::hash::Hash;

use crate::{
    backend::{Backend, DieselReserveSpecialization},
    mysql_like::query_builder::MysqlLikeQueryBuilder,
    query_builder::bind_collector::RawBytesBindCollector,
    sql_types::TypeMetadata,
};

#[cfg(any(feature = "mysql", feature = "mariadb"))]
pub use self::connection::MysqlLikeConnection;
pub use self::value::{MysqlValue, NumericRepresentation};

/// Data structures for MySQL types which have no corresponding Rust type
///
/// Most of these types are used to implement `ToSql` and `FromSql` for higher
/// level types.
pub mod data_types {
    #[doc(inline)]
    pub use super::types::date_and_time::{MysqlTime, MysqlTimestampType};
}

/// MySQL specific sql types
pub mod sql_types {
    #[doc(inline)]
    pub use super::types::{Datetime, Unsigned};
}

/// A trait for backends which implement the MySQL wire protocol. This is implemented for both MySQL and MariaDB,
/// and can be used when writing code that is compatible with both backends.
pub trait MysqlLikeBackend
where
    Self: for<'a> Backend<
            RawValue<'a> = MysqlValue<'a>,
            BindCollector<'a> = RawBytesBindCollector<Self>,
        >,
    Self: TypeMetadata<TypeMetadata = MysqlType, MetadataLookup = ()>,
    Self: Backend<QueryBuilder = MysqlLikeQueryBuilder<Self>>,
    Self: Hash + Eq + Default,
    Self: DieselReserveSpecialization,
    Self: 'static,
{
    /// The scheme used in the connection URL for this backend.
    /// "mysql" for MySQL, "mariadb" for MariaDB.
    const SCHEME: &'static str;
}

/// Represents possible types, that can be transmitted as via the
/// Mysql wire protocol
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MysqlType {
    /// A 8 bit signed integer
    Tiny,
    /// A 8 bit unsigned integer
    UnsignedTiny,
    /// A 16 bit signed integer
    Short,
    /// A 16 bit unsigned integer
    UnsignedShort,
    /// A 32 bit signed integer
    Long,
    /// A 32 bit unsigned integer
    UnsignedLong,
    /// A 64 bit signed integer
    LongLong,
    /// A 64 bit unsigned integer
    UnsignedLongLong,
    /// A 32 bit floating point number
    Float,
    /// A 64 bit floating point number
    Double,
    /// A fixed point decimal value
    Numeric,
    /// A datatype to store a time value
    Time,
    /// A datatype to store a date value
    Date,
    /// A datatype containing timestamp values ranging from
    /// '1000-01-01 00:00:00' to '9999-12-31 23:59:59'.
    DateTime,
    /// A datatype containing timestamp values ranging from
    /// 1970-01-01 00:00:01' UTC to '2038-01-19 03:14:07' UTC.
    Timestamp,
    /// A datatype for string values
    String,
    /// A datatype containing binary large objects
    Blob,
    /// A value containing a set of bit's
    Bit,
    /// A user defined set type
    Set,
    /// A user defined enum type
    Enum,
}

pub(crate) mod query_fragments {
    use crate::backend::sql_dialect::on_conflict_clause::SupportsOnConflictClause;

    #[derive(Debug, Clone, Copy)]
    pub struct MysqlStyleDefaultValueClause;

    #[derive(Debug, Clone, Copy)]
    pub struct MysqlConcatClause;

    #[derive(Debug, Clone, Copy)]
    pub struct MysqlOnConflictClause;

    #[derive(Debug, Clone, Copy)]
    pub struct MysqlRequiresOrderForWindowFunctions;

    impl SupportsOnConflictClause for MysqlOnConflictClause {}
}
