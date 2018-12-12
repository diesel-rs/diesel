//! Types and traits related to serializing values for the database

use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::result;

use backend::Backend;
use sql_types::TypeMetadata;

#[cfg(feature = "postgres")]
pub use pg::serialize::*;

/// A specialized result type representing the result of serializing
/// a value for the database.
pub type Result = result::Result<IsNull, Box<Error + Send + Sync>>;

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
#[derive(Clone, Copy)]
pub struct Output<'a, T, DB>
where
    DB: TypeMetadata,
    DB::MetadataLookup: 'a,
{
    out: T,
    metadata_lookup: &'a DB::MetadataLookup,
}

impl<'a, T, DB: TypeMetadata> Output<'a, T, DB> {
    /// Construct a new `Output`
    pub fn new(out: T, metadata_lookup: &'a DB::MetadataLookup) -> Self {
        Output {
            out,
            metadata_lookup,
        }
    }

    /// Create a new `Output` with the given buffer
    pub fn with_buffer<U>(&self, new_out: U) -> Output<'a, U, DB> {
        Output {
            out: new_out,
            metadata_lookup: self.metadata_lookup,
        }
    }

    /// Return the raw buffer this type is wrapping
    pub fn into_inner(self) -> T {
        self.out
    }

    /// Returns the backend's mechanism for dynamically looking up type
    /// metadata at runtime, if relevant for the given backend.
    pub fn metadata_lookup(&self) -> &'a DB::MetadataLookup {
        self.metadata_lookup
    }
}

#[cfg(test)]
impl<DB: TypeMetadata> Output<'static, Vec<u8>, DB> {
    /// Returns a `Output` suitable for testing `ToSql` implementations.
    /// Unsafe to use for testing types which perform dynamic metadata lookup.
    pub fn test() -> Self {
        use std::mem;
        #[allow(clippy::invalid_ref)]
        Self::new(Vec::new(), unsafe { mem::uninitialized() })
    }
}

impl<'a, T: Write, DB: TypeMetadata> Write for Output<'a, T, DB> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.out.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.out.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments) -> io::Result<()> {
        self.out.write_fmt(fmt)
    }
}

impl<'a, T, DB: TypeMetadata> Deref for Output<'a, T, DB> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.out
    }
}

impl<'a, T, DB: TypeMetadata> DerefMut for Output<'a, T, DB> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.out
    }
}

impl<'a, T, U, DB> PartialEq<U> for Output<'a, T, DB>
where
    DB: TypeMetadata,
    T: PartialEq<U>,
{
    fn eq(&self, rhs: &U) -> bool {
        self.out == *rhs
    }
}

impl<'a, T, DB> fmt::Debug for Output<'a, T, DB>
where
    T: fmt::Debug,
    DB: TypeMetadata,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
/// yourself.
///
/// Any types which implement this trait should also `#[derive(AsExpression)]`.
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
/// # use diesel::sql_types::*;
/// # use diesel::serialize::{self, ToSql, Output};
/// # use std::io::Write;
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy)]
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
///     fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
///         (*self as i32).to_sql(out)
///     }
/// }
/// ```
pub trait ToSql<A, DB: Backend>: fmt::Debug {
    /// See the trait documentation.
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> Result;
}

impl<'a, A, T, DB> ToSql<A, DB> for &'a T
where
    DB: Backend,
    T: ToSql<A, DB> + ?Sized,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> Result {
        (*self).to_sql(out)
    }
}
