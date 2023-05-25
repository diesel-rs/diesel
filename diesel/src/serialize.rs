//! Types and traits related to serializing values for the database

use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::result;

use crate::backend::Backend;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::BindCollector;

#[doc(inline)]
#[cfg(feature = "postgres_backend")]
pub use crate::pg::serialize::WriteTuple;

/// A specialized result type representing the result of serializing
/// a value for the database.
pub type Result = result::Result<IsNull, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Tiny enum to make the return type of `ToSql` more descriptive
pub enum IsNull {
    /// No data was written, as this type is null
    Yes,
    /// The value is not null
    ///
    /// This does not necessarily mean that any data was written to the buffer.
    /// For example, an empty string has no data to be sent over the wire, but
    /// also is not null.
    No,
}

/// Wraps a buffer to be written by `ToSql` with additional backend specific
/// utilities.
pub struct Output<'a, 'b, DB>
where
    DB: Backend,
    DB::MetadataLookup: 'a,
{
    out: <DB::BindCollector<'a> as BindCollector<'a, DB>>::Buffer,
    metadata_lookup: Option<&'b mut DB::MetadataLookup>,
}

impl<'a, 'b, DB: Backend> Output<'a, 'b, DB> {
    /// Construct a new `Output`
    pub fn new(
        out: <DB::BindCollector<'a> as BindCollector<'a, DB>>::Buffer,
        metadata_lookup: &'b mut DB::MetadataLookup,
    ) -> Self {
        Output {
            out,
            metadata_lookup: Some(metadata_lookup),
        }
    }

    /// Consume the current `Output` structure to access the inner buffer type
    ///
    /// This function is only useful for people implementing their own Backend.
    pub fn into_inner(self) -> <DB::BindCollector<'a> as BindCollector<'a, DB>>::Buffer {
        self.out
    }

    /// Returns the backend's mechanism for dynamically looking up type
    /// metadata at runtime, if relevant for the given backend.
    pub fn metadata_lookup(&mut self) -> &mut DB::MetadataLookup {
        self.metadata_lookup.as_mut().expect("Lookup is there")
    }

    /// Set the inner buffer to a specific value
    ///
    /// Checkout the documentation of the type of `BindCollector::Buffer`
    /// for your specific backend for supported types.
    pub fn set_value<V>(&mut self, value: V)
    where
        V: Into<<DB::BindCollector<'a> as BindCollector<'a, DB>>::Buffer>,
    {
        self.out = value.into();
    }
}

#[cfg(test)]
impl<'a, DB: Backend> Output<'a, 'static, DB> {
    /// Returns a `Output` suitable for testing `ToSql` implementations.
    /// Unsafe to use for testing types which perform dynamic metadata lookup.
    pub fn test(buffer: <DB::BindCollector<'a> as BindCollector<'a, DB>>::Buffer) -> Self {
        Self {
            out: buffer,
            metadata_lookup: None,
        }
    }
}

impl<'a, 'b, DB> Write for Output<'a, 'b, DB>
where
    for<'c> DB: Backend<BindCollector<'c> = RawBytesBindCollector<DB>>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.out.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.0.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.out.0.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.out.0.write_fmt(fmt)
    }
}

impl<'a, 'b, DB> Output<'a, 'b, DB>
where
    for<'c> DB: Backend<BindCollector<'c> = RawBytesBindCollector<DB>>,
{
    /// Call this method whenever you pass an instance of `Output<DB>` by value.
    ///
    /// Effectively copies `self`, with a narrower lifetime. When passing a
    /// reference or a mutable reference, this is normally done by rust
    /// implicitly. This is why you can pass `&mut Foo` to multiple functions,
    /// even though mutable references are not `Copy`. However, this is only
    /// done implicitly for references. For structs with lifetimes it must be
    /// done explicitly. This method matches the semantics of what Rust would do
    /// implicitly if you were passing a mutable reference
    pub fn reborrow<'c>(&'c mut self) -> Output<'c, 'c, DB>
    where
        'a: 'c,
    {
        Output {
            out: RawBytesBindCollector::<DB>::reborrow_buffer(&mut self.out),
            metadata_lookup: match &mut self.metadata_lookup {
                None => None,
                Some(m) => Some(&mut **m),
            },
        }
    }
}

impl<'a, 'b, DB> fmt::Debug for Output<'a, 'b, DB>
where
    <DB::BindCollector<'a> as BindCollector<'a, DB>>::Buffer: fmt::Debug,
    DB: Backend,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.out.fmt(f)
    }
}

/// Serializes a single value to be sent to the database.
///
/// The output is sent as a bind parameter, and the data must be written in the
/// expected format for the given backend.
///
/// When possible, implementations of this trait should prefer using an existing
/// implementation, rather than writing to `out` directly. (For example, if you
/// are implementing this for an enum, which is represented as an integer in the
/// database, you should use `i32::to_sql(x, out)` instead of writing to `out`
/// yourself.)
///
/// Any types which implement this trait should also
/// [`#[derive(AsExpression)]`](derive@crate::expression::AsExpression).
///
/// ### Backend specific details
///
/// - For PostgreSQL, the bytes will be sent using the binary protocol, not text.
/// - For SQLite, all implementations should be written in terms of an existing
///   `ToSql` implementation.
/// - For MySQL, the expected bytes will depend on the return value of
///   `type_metadata` for the given SQL type. See [`MysqlType`] for details.
/// - For third party backends, consult that backend's documentation.
///
/// [`MysqlType`]: ../mysql/enum.MysqlType.html
///
/// ### Examples
///
/// Most implementations of this trait will be defined in terms of an existing
/// implementation.
///
/// ```rust
/// # use diesel::backend::Backend;
/// # use diesel::expression::AsExpression;
/// # use diesel::sql_types::*;
/// # use diesel::serialize::{self, ToSql, Output};
/// # use std::io::Write;
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy, AsExpression)]
/// #[diesel(sql_type = Integer)]
/// pub enum MyEnum {
///     A = 1,
///     B = 2,
/// }
///
/// impl<DB> ToSql<Integer, DB> for MyEnum
/// where
///     DB: Backend,
///     i32: ToSql<Integer, DB>,
/// {
///     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
///         match self {
///             MyEnum::A => 1.to_sql(out),
///             MyEnum::B => 2.to_sql(out),
///         }
///     }
/// }
/// ```
///
/// Using temporary values as part of the `ToSql` implementation requires additional
/// work.
///
/// Backends using [`RawBytesBindCollector`] as [`BindCollector`] copy the serialized values as part
/// of `Write` implementation. This includes the `Mysql` and the `Pg` backend provided by diesel.
/// This means existing `ToSql` implementations can be used even with
/// temporary values. For these it is required to call
/// [`Output::reborrow`] to shorten the lifetime of the `Output` type correspondingly.
///
/// ```
/// # use diesel::backend::Backend;
/// # use diesel::expression::AsExpression;
/// # use diesel::sql_types::*;
/// # use diesel::serialize::{self, ToSql, Output};
/// # use std::io::Write;
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy, AsExpression)]
/// #[diesel(sql_type = Integer)]
/// pub enum MyEnum {
///     A = 1,
///     B = 2,
/// }
///
/// # #[cfg(feature = "postgres")]
/// impl ToSql<Integer, diesel::pg::Pg> for MyEnum
/// where
///     i32: ToSql<Integer, diesel::pg::Pg>,
/// {
///     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
///         let v = *self as i32;
///         <i32 as ToSql<Integer, diesel::pg::Pg>>::to_sql(&v, &mut out.reborrow())
///     }
/// }
/// ````
///
/// For any other backend the [`Output::set_value`] method provides a way to
/// set the output value directly. Checkout the documentation of the corresponding
/// `BindCollector::Buffer` type for provided `From<T>` implementations for a list
/// of accepted types. For the `Sqlite` backend see `SqliteBindValue`.
///
/// ```
/// # use diesel::backend::Backend;
/// # use diesel::expression::AsExpression;
/// # use diesel::sql_types::*;
/// # use diesel::serialize::{self, ToSql, Output, IsNull};
/// # use std::io::Write;
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy, AsExpression)]
/// #[diesel(sql_type = Integer)]
/// pub enum MyEnum {
///     A = 1,
///     B = 2,
/// }
///
/// # #[cfg(feature = "sqlite")]
/// impl ToSql<Integer, diesel::sqlite::Sqlite> for MyEnum
/// where
///     i32: ToSql<Integer, diesel::sqlite::Sqlite>,
/// {
///     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>) -> serialize::Result {
///         out.set_value(*self as i32);
///         Ok(IsNull::No)
///     }
/// }
/// ````

pub trait ToSql<A, DB: Backend>: fmt::Debug {
    /// See the trait documentation.
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> Result;
}

impl<'a, A, T, DB> ToSql<A, DB> for &'a T
where
    DB: Backend,
    T: ToSql<A, DB> + ?Sized,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> Result {
        (*self).to_sql(out)
    }
}
