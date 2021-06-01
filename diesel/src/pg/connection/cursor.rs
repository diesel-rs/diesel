use std::rc::Rc;

use super::result::PgResult;
use super::row::PgRow;

/// The type returned by various [`Conn
/// ection`] methods.
/// Acts as an iterator over `T`.
#[allow(missing_debug_implementations)]
pub struct Cursor<'a> {
    current_row: usize,
    db_result: Rc<PgResult<'a>>,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(db_result: PgResult<'a>) -> Self {
        Cursor {
            current_row: 0,
            db_result: Rc::new(db_result),
        }
    }
}

impl<'a> ExactSizeIterator for Cursor<'a> {
    fn len(&self) -> usize {
        self.db_result.num_rows() - self.current_row
    }
}

impl<'a> Iterator for Cursor<'a> {
    type Item = crate::QueryResult<PgRow<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row < self.db_result.num_rows() {
            let row = self.db_result.clone().get_row(self.current_row);
            self.current_row += 1;
            Some(Ok(row))
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

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}
