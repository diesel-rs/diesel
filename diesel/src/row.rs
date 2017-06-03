use backend::Backend;

/// The row trait which is used for [`FromSqlRow`][]. Apps should not need to
/// concern themselves with this trait.
///
/// [`FromSqlRow`]: ../types/trait.FromSqlRow.html
pub trait Row<DB: Backend> {
    fn take(&mut self) -> Option<&DB::RawValue>;
    fn next_is_null(&self, count: usize) -> bool;

    fn advance(&mut self, count: usize) {
        for _ in 0..count {
            self.take();
        }
    }
}
