use super::result::PgResult;
use super::row::PgRow;

/// The type returned by various [`Connection`](struct.Connection.html) methods.
/// Acts as an iterator over `T`.
pub struct Cursor<'a> {
    current_row: usize,
    db_result: &'a PgResult,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(db_result: &'a PgResult) -> Self {
        Cursor {
            current_row: 0,
            db_result,
        }
    }
}

impl<'a> ExactSizeIterator for Cursor<'a> {}

impl<'a> Iterator for Cursor<'a> {
    type Item = PgRow<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row >= self.db_result.num_rows() {
            None
        } else {
            let row = self.db_result.get_row(self.current_row);
            self.current_row += 1;

            Some(row)
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.current_row += n;
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.db_result.num_rows();
        (len, Some(len))
    }
}
