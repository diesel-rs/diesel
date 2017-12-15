//! Types which represent various database backends

use byteorder::ByteOrder;

use query_builder::QueryBuilder;
use query_builder::bind_collector::BindCollector;
use types::{self, HasSqlType};

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
    Self: HasSqlType<types::SmallInt>,
    Self: HasSqlType<types::Integer>,
    Self: HasSqlType<types::BigInt>,
    Self: HasSqlType<types::Float>,
    Self: HasSqlType<types::Double>,
    Self: HasSqlType<types::VarChar>,
    Self: HasSqlType<types::Text>,
    Self: HasSqlType<types::Binary>,
    Self: HasSqlType<types::Date>,
    Self: HasSqlType<types::Time>,
    Self: HasSqlType<types::Timestamp>,
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

/// Information about how a backend stores metadata about given SQL types
pub trait TypeMetadata {
    /// The actual type used to represent metadata.
    ///
    /// On PostgreSQL, this is the type's OID.
    /// On MySQL and SQLite, this is an enum representing all storage classes
    /// they support.
    type TypeMetadata;
    /// The type used for runtime lookup of metadata.
    ///
    /// For most backends, which don't support user defined types, this will
    /// be `()`.
    type MetadataLookup;
}

/// Does this backend support `RETURNING` clauses?
pub trait SupportsReturningClause {}
/// Does this backend support the bare `DEFAULT` keyword?
pub trait SupportsDefaultKeyword {}
/// Does this backend use the standard `SAVEPOINT` syntax?
pub trait UsesAnsiSavepointSyntax {}
