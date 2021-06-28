use std::cell::RefCell;
use std::rc::Rc;

use super::row::{PrivateSqliteRow, SqliteRow};
use super::stmt::StatementUse;
use crate::result::QueryResult;

#[allow(missing_debug_implementations)]
pub struct StatementIterator<'a> {
    inner: PrivateStatementIterator<'a>,
    column_names: Option<Rc<Vec<Option<String>>>>,
    field_count: usize,
}

enum PrivateStatementIterator<'a> {
    NotStarted(StatementUse<'a>),
    Started(Rc<RefCell<PrivateSqliteRow<'a>>>),
    TemporaryEmpty,
}

impl<'a> StatementIterator<'a> {
    pub fn new(stmt: StatementUse<'a>) -> Self {
        Self {
            inner: PrivateStatementIterator::NotStarted(stmt),
            column_names: None,
            field_count: 0,
        }
    }
}

impl<'a> Iterator for StatementIterator<'a> {
    type Item = QueryResult<SqliteRow<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        use PrivateStatementIterator::*;

        match std::mem::replace(&mut self.inner, TemporaryEmpty) {
            NotStarted(stmt) => match stmt.step() {
                Err(e) => Some(Err(e)),
                Ok(None) => None,
                Ok(Some(stmt)) => {
                    let field_count = stmt.column_count() as usize;
                    self.field_count = field_count;
                    let inner = Rc::new(RefCell::new(PrivateSqliteRow::Direct(stmt)));
                    self.inner = Started(inner.clone());
                    Some(Ok(SqliteRow { inner, field_count }))
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
                                let field_count = self.field_count;
                                (*last_row_ref.get_mut()) = PrivateSqliteRow::Direct(stmt);
                                self.inner = Started(last_row.clone());
                                Some(Ok(SqliteRow {
                                    inner: last_row,
                                    field_count,
                                }))
                            }
                        }
                    } else {
                        // any other state than `PrivateSqliteRow::Direct` is invalid here
                        // and should not happen. If this ever happens this is a logic error
                        // in the code above
                        unreachable!(
                            "You've reached an impossible internal state. \
                             If you ever see this error message please open \
                             an issue at https://github.com/diesel-rs/diesel \
                             providing example code how to trigger this error."
                        )
                    }
                } else {
                    // We don't own the statement. There is another existing reference, likly because
                    // a user stored the row in some long time container before calling next another time
                    // In this case we copy out the current values into a temporary store and advance
                    // the statement iterator internally afterwards
                    let last_row = {
                        let mut last_row = match last_row.try_borrow_mut() {
                            Ok(o) => o,
                            Err(_e) => {
                                self.inner = Started(last_row.clone());
                                return Some(Err(crate::result::Error::DeserializationError(
                                    "Failed to reborrow row. Try to release any `SqliteValue` \
                                     that exists at this point"
                                        .into(),
                                )));
                            }
                        };
                        let last_row = &mut *last_row;
                        let duplicated = last_row.duplicate(&mut self.column_names);
                        std::mem::replace(last_row, duplicated)
                    };
                    if let PrivateSqliteRow::Direct(stmt) = last_row {
                        match stmt.step() {
                            Err(e) => Some(Err(e)),
                            Ok(None) => None,
                            Ok(Some(stmt)) => {
                                let field_count = self.field_count;
                                let last_row =
                                    Rc::new(RefCell::new(PrivateSqliteRow::Direct(stmt)));
                                self.inner = Started(last_row.clone());
                                Some(Ok(SqliteRow {
                                    inner: last_row,
                                    field_count,
                                }))
                            }
                        }
                    } else {
                        // any other state than `PrivateSqliteRow::Direct` is invalid here
                        // and should not happen. If this ever happens this is a logic error
                        // in the code above
                        unreachable!(
                            "You've reached an impossible internal state. \
                             If you ever see this error message please open \
                             an issue at https://github.com/diesel-rs/diesel \
                             providing example code how to trigger this error."
                        )
                    }
                }
            }
            TemporaryEmpty => None,
        }
    }
}
