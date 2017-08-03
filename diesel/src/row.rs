//! Contains the `Row` trait

use backend::Backend;

/// The row trait which is used for [`FromSqlRow`][]. Apps should not need to
/// concern themselves with this trait.
///
/// [`FromSqlRow`]: ../types/trait.FromSqlRow.html
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
