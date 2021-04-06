use super::result::PgResult;
use super::row::PgRow;

/// The type returned by various [`Connection`] methods.
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

impl<'a> ExactSizeIterator for Cursor<'a> {
    fn len(&self) -> usize {
        self.db_result.num_rows() - self.current_row
    }
}

impl<'a> Iterator for Cursor<'a> {
    type Item = PgRow<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row < self.db_result.num_rows() {
            let row = self.db_result.get_row(self.current_row);
            self.current_row += 1;
            Some(row)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.current_row = (self.current_row + n).min(self.db_result.num_rows());
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
