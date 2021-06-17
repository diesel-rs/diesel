use std::cell::RefCell;
use std::rc::Rc;

use super::row::{PrivateSqliteRow, SqliteRow};
use super::stmt::StatementUse;
use crate::result::QueryResult;

#[allow(missing_debug_implementations)]
pub struct StatementIterator<'a: 'b, 'b> {
    inner: PrivateStatementIterator<'a, 'b>,
    column_names: Option<Rc<Vec<Option<String>>>>,
}

enum PrivateStatementIterator<'a: 'b, 'b> {
    NotStarted(StatementUse<'a, 'b>),
    Started(Rc<RefCell<PrivateSqliteRow<'a, 'b>>>),
    TemporaryEmpty,
}

impl<'a: 'b, 'b> StatementIterator<'a, 'b> {
    pub fn new(stmt: StatementUse<'a, 'b>) -> Self {
        Self {
            inner: PrivateStatementIterator::NotStarted(stmt),
            column_names: None,
        }
    }
}

impl<'a: 'b, 'b> Iterator for StatementIterator<'a, 'b> {
    type Item = QueryResult<SqliteRow<'a, 'b>>;

    fn next(&mut self) -> Option<Self::Item> {
        use PrivateStatementIterator::*;

        match std::mem::replace(&mut self.inner, TemporaryEmpty) {
            NotStarted(stmt) => match stmt.step() {
                Err(e) => Some(Err(e)),
                Ok(None) => None,
                Ok(Some(row)) => {
                    let inner = Rc::new(RefCell::new(PrivateSqliteRow::Direct(row)));
                    self.inner = Started(inner.clone());
                    Some(Ok(SqliteRow { inner }))
                }
            },
            Started(mut last_row) => {
                // There was already at least one iteration step
                // We check here if the caller already released the row value or not
                // by checking if our Rc owns the data or not
                if let Some(last_row_ref) = Rc::get_mut(&mut last_row) {
                    // We own the statement, there is no other reference here.
                    // This means we don't need to copy out values from the sqlite provided
                    // datastructures for now
                    // We don't need to use the runtime borrowing system of the RefCell here
                    // as we have a mutable reference, so all of this below is checked at compile time
                    if let PrivateSqliteRow::Direct(stmt) =
                        std::mem::replace(last_row_ref.get_mut(), PrivateSqliteRow::TemporaryEmpty)
                    {
                        match stmt.step() {
                            Err(e) => Some(Err(e)),
                            Ok(None) => None,
                            Ok(Some(stmt)) => {
                                (*last_row_ref.get_mut()) = PrivateSqliteRow::Direct(stmt);
                                self.inner = Started(last_row.clone());
                                Some(Ok(SqliteRow { inner: last_row }))
                            }
                        }
                    } else {
                        // any other state than `PrivateSqliteRow::Direct` is invalid here
                        // and should not happen. If this ever happens this is a logic error
                        // in the code above
                        unreachable!()
                    }
                } else {
                    // We don't own the statement. There is another existing reference, likly because
                    // a user stored the row in some long time container before calling next another time
                    // In this case we copy out the current values into a temporary store and advance
                    // the statement iterator internally afterwards
                    if let PrivateSqliteRow::Direct(stmt) =
                        last_row.replace_with(|inner| inner.duplicate(&mut self.column_names))
                    {
                        match stmt.step() {
                            Err(e) => Some(Err(e)),
                            Ok(None) => None,
                            Ok(Some(stmt)) => {
                                let last_row =
                                    Rc::new(RefCell::new(PrivateSqliteRow::Direct(stmt)));
                                self.inner = Started(last_row.clone());
                                Some(Ok(SqliteRow { inner: last_row }))
                            }
                        }
                    } else {
                        // any other state than `PrivateSqliteRow::Direct` is invalid here
                        // and should not happen. If this ever happens this is a logic error
                        // in the code above
                        unreachable!()
                    }
                }
            }
            TemporaryEmpty => None,
        }
    }
}
