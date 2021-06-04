//! Types which represent various database backends

use byteorder::ByteOrder;

use crate::query_builder::bind_collector::BindCollector;
use crate::query_builder::QueryBuilder;
use crate::sql_types::{self, HasSqlType};

/// A database backend
///
/// This trait represents the concept of a backend (e.g. "MySQL" vs "SQLite").
/// It is separate from a [`Connection`](crate::connection::Connection)
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
    Self: for<'a> HasRawValue<'a>,
{
    /// The concrete `QueryBuilder` implementation for this backend.
    type QueryBuilder: QueryBuilder<Self>;
    /// The concrete `BindCollector` implementation for this backend.
    ///
    /// Most backends should use [`RawBytesBindCollector`].
    ///
    /// [`RawBytesBindCollector`]: crate::query_builder::bind_collector::RawBytesBindCollector
    type BindCollector: BindCollector<Self>;
    /// What byte order is used to transmit integers?
    ///
    /// This type is only used if `RawValue` is `[u8]`.
    type ByteOrder: ByteOrder;
}

/// The raw representation of a database value given to `FromSql`.
///
/// This trait is separate from `Backend` to imitate `type RawValue<'a>`. It
/// should only be referenced directly by implementors. Users of this type
/// should instead use the [`RawValue`] helper type instead.
pub trait HasRawValue<'a> {
    /// The actual type given to `FromSql`, with lifetimes applied. This type
    /// should not be used directly. Use the [`RawValue`]
    /// helper type instead.
    type RawValue;
}

/// A trait indicating that the provided raw value uses a binary representation internally
// That's a false positive, `HasRawValue<'a>` is essentially
// a reference wrapper
#[allow(clippy::wrong_self_convention)]
pub trait BinaryRawValue<'a>: HasRawValue<'a> {
    /// Get the underlying binary representation of the raw value
    fn as_bytes(value: Self::RawValue) -> &'a [u8];
}

/// A helper type to get the raw representation of a database type given to
/// `FromSql`. Equivalent to `<DB as Backend>::RawValue<'a>`.
pub type RawValue<'a, DB> = <DB as HasRawValue<'a>>::RawValue;

/// A trait indicating that implementing backend provides support for
/// `RETURNING` clauses.
///
/// This trait has to be implemented in order to be able to use methods such as
/// `get_results` and `returning`. Namely, any method which leads to usage of
/// `RETURNING` clauses in SQL sent to backend will require that this trait
/// be implemented for used backend.
pub trait SupportsReturningClause {}
/// Does this backend support 'ON CONFLICT' clause?
pub trait SupportsOnConflictClause {}
/// Does this backend support 'WHERE' clauses on 'ON CONFLICT' clauses?
pub trait SupportsOnConflictTargetDecorations {}
/// Does this backend support the bare `DEFAULT` keyword?
pub trait SupportsDefaultKeyword {}
/// Does this backend use the standard `SAVEPOINT` syntax?
pub trait UsesAnsiSavepointSyntax {}
