//! Types which represent various database backends

use byteorder::ByteOrder;

use crate::query_builder::bind_collector::BindCollector;
use crate::query_builder::QueryBuilder;
use crate::sql_types::{self, HasSqlType};

/// A database backend
///
/// This trait represents the concept of a backend (e.g. "MySQL" vs "SQLite").
/// It is separate from a [`Connection`](../connection/trait.Connection.html)
/// to that backend.
/// One backend may have multiple concrete connection implementations.
///
/// Implementations of this trait should not assume details about how the
/// connection is implemented.
/// For example, the `Pg` backend does not assume that `libpq` is being used.
/// Implementations of this trait can and should care about details of the wire
/// protocol used to communicated with the database.
pub trait Backend
where
    Self: Sized,
    Self: HasSqlType<sql_types::SmallInt>,
    Self: HasSqlType<sql_types::Integer>,
    Self: HasSqlType<sql_types::BigInt>,
    Self: HasSqlType<sql_types::Float>,
    Self: HasSqlType<sql_types::Double>,
    Self: HasSqlType<sql_types::VarChar>,
    Self: HasSqlType<sql_types::Text>,
    Self: HasSqlType<sql_types::Binary>,
    Self: HasSqlType<sql_types::Date>,
    Self: HasSqlType<sql_types::Time>,
    Self: HasSqlType<sql_types::Timestamp>,
{
    /// The concrete `QueryBuilder` implementation for this backend.
    type QueryBuilder: QueryBuilder<Self>;
    /// The concrete `BindCollector` implementation for this backend.
    ///
    /// Most backends should use [`RawBytesBindCollector`].
    ///
    /// [`RawBytesBindCollector`]: ../query_builder/bind_collector/struct.RawBytesBindCollector.html
    type BindCollector: BindCollector<Self>;
    /// The raw representation of a database value given to `FromSql`.
    ///
    /// Since most backends transmit data as opaque blobs of bytes, this type
    /// is usually `[u8]`.
    type RawValue: ?Sized;
    /// What byte order is used to transmit integers?
    ///
    /// This type is only used if `RawValue` is `[u8]`.
    type ByteOrder: ByteOrder;
}

/// Does this backend support `RETURNING` clauses?
pub trait SupportsReturningClause {}
/// Does this backend support the bare `DEFAULT` keyword?
pub trait SupportsDefaultKeyword {}
/// Does this backend use the standard `SAVEPOINT` syntax?
pub trait UsesAnsiSavepointSyntax {}

#[cfg(feature = "with-deprecated")]
#[deprecated(since = "1.1.0", note = "use `sql_types::TypeMetadata` instead")]
pub use crate::sql_types::TypeMetadata;
