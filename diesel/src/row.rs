//! Contains the `Row` trait

use crate::backend::Backend;
use crate::deserialize;
use deserialize::FromSql;
use std::default::Default;
use std::ops::Range;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
#[doc(inline)]
pub use self::private::{PartialRow, RowSealed};

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
#[allow(unused_imports)]
pub(crate) use self::private::{PartialRow, RowSealed};

/// Representing a way to index into database rows
///
/// * Crates using existing backends should use existing implementations of
///   this traits. Diesel provides `RowIndex<usize>` and `RowIndex<&str>` for
///   all built-in backends
///
/// * Crates implementing custom backends need to provide `RowIndex<usize>` and
///   `RowIndex<&str>` impls for their [`Row`] type.
///
pub trait RowIndex<I> {
    /// Get the numeric index inside the current row for the provided index value
    fn idx(&self, idx: I) -> Option<usize>;
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(note = "Use `Row::Field` directly instead")]
pub type FieldRet<'a, R, DB> = <R as self::private::RowLifetimeHelper<DB>>::Field<'a>;

/// Represents a single database row.
///
/// This trait is used as an argument to [`FromSqlRow`].
///
/// [`FromSqlRow`]: crate::deserialize::FromSqlRow
pub trait Row<'a, DB: Backend>:
    RowIndex<usize> + for<'b> RowIndex<&'b str> + self::private::RowSealed + Sized
{
    /// Field type returned by a `Row` implementation
    ///
    /// * Crates using existing backends should not concern themself with the
    ///   concrete type of this associated type.
    ///
    /// * Crates implementing custom backends should provide their own type
    ///   meeting the required trait bounds
    type Field<'f>: Field<'f, DB>
    where
        'a: 'f,
        Self: 'f;

    /// Return type of `PartialRow`
    ///
    /// For all implementations, beside of the `Row` implementation on `PartialRow` itself
    /// this should be `Self`.
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc(hidden)
    )]
    type InnerPartialRow: Row<'a, DB>;

    /// Get the number of fields in the current row
    fn field_count(&self) -> usize;

    /// Get the field with the provided index from the row.
    ///
    /// Returns `None` if there is no matching field for the given index
    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: RowIndex<I>;

    /// Get a deserialized value with the provided index from the row.
    fn get_value<ST, T, I>(&self, idx: I) -> crate::deserialize::Result<T>
    where
        Self: RowIndex<I>,
        T: FromSql<ST, DB>,
    {
        let field = self.get(idx).ok_or(crate::result::UnexpectedEndOfRow)?;
        <T as FromSql<ST, DB>>::from_nullable_sql(field.value())
    }

    /// Returns a wrapping row that allows only to access fields, where the index is part of
    /// the provided range.
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc(hidden)
    )]
    fn partial_row(&self, range: Range<usize>) -> PartialRow<'_, Self::InnerPartialRow>;
}

/// Represents a single field in a database row.
///
/// This trait allows retrieving information on the name of the column and on the value of the
/// field.
pub trait Field<'a, DB: Backend> {
    /// The name of the current field
    ///
    /// Returns `None` if it's an unnamed field
    fn field_name(&self) -> Option<&str>;

    /// Get the value representing the current field in the raw representation
    /// as it is transmitted by the database
    fn value(&self) -> Option<DB::RawValue<'_>>;

    /// Checks whether this field is null or not.
    fn is_null(&self) -> bool {
        self.value().is_none()
    }
}

/// Represents a row of a SQL query, where the values are accessed by name
/// rather than by index.
///
/// This trait is used by implementations of
/// [`QueryableByName`](crate::deserialize::QueryableByName)
pub trait NamedRow<'a, DB: Backend>: Row<'a, DB> {
    /// Retrieve and deserialize a single value from the query
    ///
    /// Note that `ST` *must* be the exact type of the value with that name in
    /// the query. The compiler will not be able to verify that you have
    /// provided the correct type. If there is a mismatch, you may receive an
    /// incorrect value, or a runtime error.
    ///
    /// If two or more fields in the query have the given name, the result of
    /// this function is undefined.
    fn get<ST, T>(&self, column_name: &str) -> deserialize::Result<T>
    where
        T: FromSql<ST, DB>;
}

impl<'a, R, DB> NamedRow<'a, DB> for R
where
    R: Row<'a, DB>,
    DB: Backend,
{
    fn get<ST, T>(&self, column_name: &str) -> deserialize::Result<T>
    where
        T: FromSql<ST, DB>,
    {
        let field = Row::get(self, column_name)
            .ok_or_else(|| format!("Column `{column_name}` was not present in query"))?;

        T::from_nullable_sql(field.value())
    }
}

/// A row that can be turned into an owned version
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub trait IntoOwnedRow<'a, DB: Backend>: Row<'a, DB> {
    /// The owned version of the row
    type OwnedRow: Row<'a, DB> + Send + 'static;

    /// A store for cached information between rows for faster access
    type Cache: Default + 'static;

    /// Turn the row into its owned version
    fn into_owned(self, cache: &mut Self::Cache) -> Self::OwnedRow;
}

// These traits are not part of the public API
// because:
// * we want to control who can implement `Row` (for `RowSealed`)
// * `PartialRow` is an implementation detail
// * `RowLifetimeHelper` is an internal backward compatibility helper
pub(crate) mod private {
    use super::*;

    /// This trait restricts who can implement `Row`
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub trait RowSealed {}

    /// A row type that wraps an inner row
    ///
    /// This type only allows to access fields of the inner row, whose index is
    /// part of `range`. This type is used by diesel internally to implement
    /// [`FromStaticSqlRow`](crate::deserialize::FromStaticSqlRow).
    ///
    /// Indexing via `usize` starts with 0 for this row type. The index is then shifted
    /// by `self.range.start` to match the corresponding field in the underlying row.
    #[derive(Debug)]
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
    )]
    pub struct PartialRow<'a, R> {
        inner: &'a R,
        range: Range<usize>,
    }

    impl<'a, R> PartialRow<'a, R> {
        /// Create a new [`PartialRow`] instance based on an inner
        /// row and a range of field that should be part of the constructed
        /// wrapper.
        ///
        /// See the documentation of [`PartialRow`] for details.
        pub fn new<'b, DB>(inner: &'a R, range: Range<usize>) -> Self
        where
            R: Row<'b, DB>,
            DB: Backend,
        {
            let range_lower = std::cmp::min(range.start, inner.field_count());
            let range_upper = std::cmp::min(range.end, inner.field_count());
            Self {
                inner,
                range: range_lower..range_upper,
            }
        }
    }

    impl<'a, R> RowSealed for PartialRow<'a, R> {}

    impl<'a, 'b, DB, R> Row<'a, DB> for PartialRow<'b, R>
    where
        DB: Backend,
        R: Row<'a, DB>,
    {
        type Field<'f> = R::Field<'f> where 'a: 'f, R: 'f, Self: 'f;
        type InnerPartialRow = R;

        fn field_count(&self) -> usize {
            self.range.len()
        }

        fn get<'c, I>(&'c self, idx: I) -> Option<Self::Field<'c>>
        where
            'a: 'c,
            Self: RowIndex<I>,
        {
            let idx = self.idx(idx)?;
            self.inner.get(idx)
        }

        fn partial_row(&self, range: Range<usize>) -> PartialRow<'_, R> {
            let range_upper_bound = std::cmp::min(self.range.end, self.range.start + range.end);
            let range = (self.range.start + range.start)..range_upper_bound;
            PartialRow {
                inner: self.inner,
                range,
            }
        }
    }

    impl<'a, 'b, R> RowIndex<&'a str> for PartialRow<'b, R>
    where
        R: RowIndex<&'a str>,
    {
        fn idx(&self, idx: &'a str) -> Option<usize> {
            let idx = self.inner.idx(idx)?;
            if self.range.contains(&idx) {
                Some(idx)
            } else {
                None
            }
        }
    }

    impl<'a, R> RowIndex<usize> for PartialRow<'a, R>
    where
        R: RowIndex<usize>,
    {
        fn idx(&self, idx: usize) -> Option<usize> {
            let idx = self.inner.idx(idx + self.range.start)?;
            if self.range.contains(&idx) {
                Some(idx)
            } else {
                None
            }
        }
    }

    // These impls are only there for backward compatibility reasons
    // Remove them on the next breaking release
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    #[allow(unreachable_pub)]
    pub trait RowLifetimeHelper<DB>: for<'a> super::Row<'a, DB>
    where
        DB: Backend,
    {
        type Field<'f>
        where
            Self: 'f;
    }

    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    impl<R, DB> RowLifetimeHelper<DB> for R
    where
        DB: Backend,
        for<'a> R: super::Row<'a, DB>,
    {
        type Field<'f> = <R as super::Row<'f, DB>>::Field<'f> where R: 'f;
    }
}
