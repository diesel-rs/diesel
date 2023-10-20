use std::cell::RefCell;
use std::rc::Rc;

use super::row::{PrivateSqliteRow, SqliteRow};
use super::stmt::StatementUse;
use crate::result::QueryResult;

#[allow(missing_debug_implementations)]
pub struct StatementIterator<'stmt, 'query> {
    inner: PrivateStatementIterator<'stmt, 'query>,
    column_names: Option<Rc<[Option<String>]>>,
    field_count: usize,
}

impl<'stmt, 'query> StatementIterator<'stmt, 'query> {
    #[cold]
    #[allow(unsafe_code)] // call to unsafe function
    fn handle_duplicated_row_case(
        outer_last_row: &mut Rc<RefCell<PrivateSqliteRow<'stmt, 'query>>>,
        column_names: &mut Option<Rc<[Option<String>]>>,
        field_count: usize,
    ) -> Option<QueryResult<SqliteRow<'stmt, 'query>>> {
        // We don't own the statement. There is another existing reference, likely because
        // a user stored the row in some long time container before calling next another time
        // In this case we copy out the current values into a temporary store and advance
        // the statement iterator internally afterwards
        let last_row = {
            let mut last_row = match outer_last_row.try_borrow_mut() {
                Ok(o) => o,
                Err(_e) => {
                    return Some(Err(crate::result::Error::DeserializationError(
                                    "Failed to reborrow row. Try to release any `SqliteField` or `SqliteValue` \
                                     that exists at this point"
                                        .into(),
                                )));
                }
            };
            let last_row = &mut *last_row;
            let duplicated = last_row.duplicate(column_names);
            std::mem::replace(last_row, duplicated)
        };
        if let PrivateSqliteRow::Direct(mut stmt) = last_row {
            let res = unsafe {
                // This is actually safe here as we've already
                // performed one step. For the first step we would have
                // used `PrivateStatementIterator::NotStarted` where we don't
                // have access to `PrivateSqliteRow` at all
                stmt.step(false)
            };
            *outer_last_row = Rc::new(RefCell::new(PrivateSqliteRow::Direct(stmt)));
            match res {
                Err(e) => Some(Err(e)),
                Ok(false) => None,
                Ok(true) => Some(Ok(SqliteRow {
                    inner: Rc::clone(outer_last_row),
                    field_count,
                })),
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

enum PrivateStatementIterator<'stmt, 'query> {
    NotStarted(Option<StatementUse<'stmt, 'query>>),
    Started(Rc<RefCell<PrivateSqliteRow<'stmt, 'query>>>),
}

impl<'stmt, 'query> StatementIterator<'stmt, 'query> {
    pub fn new(stmt: StatementUse<'stmt, 'query>) -> StatementIterator<'stmt, 'query> {
        Self {
            inner: PrivateStatementIterator::NotStarted(Some(stmt)),
            column_names: None,
            field_count: 0,
        }
    }
}

impl<'stmt, 'query> Iterator for StatementIterator<'stmt, 'query> {
    type Item = QueryResult<SqliteRow<'stmt, 'query>>;

    #[allow(unsafe_code)] // call to unsafe function
    fn next(&mut self) -> Option<Self::Item> {
        use PrivateStatementIterator::{NotStarted, Started};
        match &mut self.inner {
            NotStarted(ref mut stmt) if stmt.is_some() => {
                let mut stmt = stmt
                    .take()
                    .expect("It must be there because we checked that above");
                let step = unsafe {
                    // This is safe as we pass `first_step = true` to reset the cached column names
                    stmt.step(true)
                };
                match step {
                    Err(e) => Some(Err(e)),
                    Ok(false) => None,
                    Ok(true) => {
                        let field_count = stmt.column_count() as usize;
                        self.field_count = field_count;
                        let inner = Rc::new(RefCell::new(PrivateSqliteRow::Direct(stmt)));
                        self.inner = Started(inner.clone());
                        Some(Ok(SqliteRow { inner, field_count }))
                    }
                }
            }
            Started(ref mut last_row) => {
                // There was already at least one iteration step
                // We check here if the caller already released the row value or not
                // by checking if our Rc owns the data or not
                if let Some(last_row_ref) = Rc::get_mut(last_row) {
                    // We own the statement, there is no other reference here.
                    // This means we don't need to copy out values from the sqlite provided
                    // datastructures for now
                    // We don't need to use the runtime borrowing system of the RefCell here
                    // as we have a mutable reference, so all of this below is checked at compile time
                    if let PrivateSqliteRow::Direct(ref mut stmt) = last_row_ref.get_mut() {
                        let step = unsafe {
                            // This is actually safe here as we've already
                            // performed one step. For the first step we would have
                            // used `PrivateStatementIterator::NotStarted` where we don't
                            // have access to `PrivateSqliteRow` at all

                            stmt.step(false)
                        };
                        match step {
                            Err(e) => Some(Err(e)),
                            Ok(false) => None,
                            Ok(true) => {
                                let field_count = self.field_count;
                                Some(Ok(SqliteRow {
                                    inner: Rc::clone(last_row),
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
                    Self::handle_duplicated_row_case(
                        last_row,
                        &mut self.column_names,
                        self.field_count,
                    )
                }
            }
            NotStarted(_) => unreachable!(
                "You've reached an impossible internal state. \
                             If you ever see this error message please open \
                             an issue at https://github.com/diesel-rs/diesel \
                             providing example code how to trigger this error."
            ),
        }
    }
}
