//! Contains the `Row` trait

use crate::backend::Backend;
use crate::deserialize::{self, FromSql};

/// Represents a single database row.
/// Apps should not need to concern themselves with this trait.
///
/// This trait is only used as an argument to [`FromSqlRow`].
///
/// [`FromSqlRow`]: ../deserialize/trait.FromSqlRow.html
pub trait Row<DB: Backend> {
    /// Returns the value of the next column in the row.
    fn take(&mut self) -> Option<&DB::RawValue>;

    /// Returns whether the next `count` columns are all `NULL`.
    ///
    /// If this method returns `true`, then the next `count` calls to `take`
    /// would all return `None`.
    fn next_is_null(&self, count: usize) -> bool;

    /// Skips the next `count` columns. This method must be called if you are
    /// choosing not to call `take` as a result of `next_is_null` returning
    /// `true`.
    fn advance(&mut self, count: usize) {
        for _ in 0..count {
            self.take();
        }
    }
}

/// Represents a row of a SQL query, where the values are accessed by name
/// rather than by index.
///
/// This trait is used by implementations of
/// [`QueryableByName`](../deserialize/trait.QueryableByName.html)
pub trait NamedRow<DB: Backend> {
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
        T: FromSql<ST, DB>,
    {
        let idx = self
            .index_of(column_name)
            .ok_or_else(|| format!("Column `{}` was not present in query", column_name).into());
        let idx = match idx {
            Ok(x) => x,
            Err(e) => return Err(e),
        };
        let raw_value = self.get_raw_value(idx);
        T::from_sql(raw_value)
    }

    #[doc(hidden)]
    fn index_of(&self, column_name: &str) -> Option<usize>;
    #[doc(hidden)]
    fn get_raw_value(&self, index: usize) -> Option<&DB::RawValue>;
}
