use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use core::iter::FusedIterator;

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
    ) -> Option<(QueryResult<SqliteRow<'stmt, 'query>>, bool)> {
        // We don't own the statement. There is another existing reference, likely because
        // a user stored the row in some long time container before calling next another time
        // In this case we copy out the current values into a temporary store and advance
        // the statement iterator internally afterwards
        let last_row = {
            let mut last_row = match outer_last_row.try_borrow_mut() {
                Ok(o) => o,
                Err(_e) => {
                    return Some((Err(crate::result::Error::DeserializationError(
                                    "Failed to reborrow row. Try to release any `SqliteField` or `SqliteValue` \
                                     that exists at this point"
                                        .into(),
                                )), false));
                }
            };
            let last_row = &mut *last_row;
            let duplicated = match last_row.duplicate(column_names) {
                Ok(duplicated) => duplicated,
                // The statement is not advanced, so the row it holds stays current.
                Err(e) => return Some((Err(e), false)),
            };
            core::mem::replace(last_row, duplicated)
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
                Err(e) => Some((Err(e), true)),
                Ok(false) => None,
                Ok(true) => Some((
                    Ok(SqliteRow {
                        inner: Rc::clone(outer_last_row),
                        field_count,
                    }),
                    false,
                )),
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
    Finished,
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
        use PrivateStatementIterator::{Finished, NotStarted, Started};
        match &mut self.inner {
            NotStarted(stmt @ Some(_)) => {
                let mut stmt = stmt
                    .take()
                    .expect("It must be there because we checked that above");
                let step = unsafe {
                    // This is safe as we pass `first_step = true` to reset the cached column names
                    stmt.step(true)
                };
                match step {
                    Err(e) => {
                        self.inner = Finished;
                        Some(Err(e))
                    }
                    Ok(false) => {
                        // we reached the end of the iterator
                        // so we mark it done on rust side
                        self.inner = Finished;
                        None
                    }
                    Ok(true) => {
                        let field_count = stmt
                            .column_count()
                            .try_into()
                            .expect("Diesel expects to run at least on a 32 bit platform");
                        self.field_count = field_count;
                        let inner = Rc::new(RefCell::new(PrivateSqliteRow::Direct(stmt)));
                        self.inner = Started(inner.clone());
                        Some(Ok(SqliteRow { inner, field_count }))
                    }
                }
            }
            Started(last_row) => {
                // There was already at least one iteration step
                // We check here if the caller already released the row value or not
                // by checking if our Rc owns the data or not
                if let Some(last_row_ref) = Rc::get_mut(last_row) {
                    // We own the statement, there is no other reference here.
                    // This means we don't need to copy out values from the sqlite provided
                    // datastructures for now
                    // We don't need to use the runtime borrowing system of the RefCell here
                    // as we have a mutable reference, so all of this below is checked at compile time
                    if let PrivateSqliteRow::Direct(stmt) = last_row_ref.get_mut() {
                        let step = unsafe {
                            // This is actually safe here as we've already
                            // performed one step. For the first step we would have
                            // used `PrivateStatementIterator::NotStarted` where we don't
                            // have access to `PrivateSqliteRow` at all

                            stmt.step(false)
                        };
                        match step {
                            Err(e) => {
                                self.inner = Finished;
                                Some(Err(e))
                            }
                            Ok(false) => {
                                // we reached the end of the iterator
                                // so we mark it done on rust side
                                self.inner = Finished;
                                None
                            }
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
                    let result = Self::handle_duplicated_row_case(
                        last_row,
                        &mut self.column_names,
                        self.field_count,
                    );
                    if result.as_ref().is_none_or(|(r, f)| *f && r.is_err()) {
                        self.inner = Finished;
                    }
                    result.map(|(r, _)| r)
                }
            }
            NotStarted(_s) => {
                // we likely got an error while executing the other
                // `NotStarted` branch above. In this case we just want to stop
                // iterating here
                None
            }
            Finished => None,
        }
    }
}

impl<'stmt, 'query> FusedIterator for StatementIterator<'stmt, 'query> {}

#[cfg(test)]
mod tests {
    use crate::connection::{DefaultLoadingMode, SimpleConnection};
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::Integer;

    #[diesel_test_helper::test]
    fn statement_iterator_is_fused() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute("CREATE TABLE t(x INTEGER); INSERT INTO t VALUES (1), (2);")
            .unwrap();
        {
            let mut it = sql::<Integer>("SELECT x FROM t")
                .load_iter::<i32, DefaultLoadingMode>(&mut conn)
                .unwrap();
            assert_eq!(it.next().unwrap().unwrap(), 1);
            assert_eq!(it.next().unwrap().unwrap(), 2);
            assert!(it.next().is_none());
            // Polling past the end must not restart (re-execute) the statement.
            assert!(it.next().is_none(), "iterator re-executed the statement");
        }
        conn.batch_execute("DELETE FROM t").unwrap();
        let mut it = sql::<Integer>("SELECT x FROM t")
            .load_iter::<i32, DefaultLoadingMode>(&mut conn)
            .unwrap();
        assert!(it.next().is_none());
        // Polling past the end must not restart (re-execute) the statement.
        assert!(it.next().is_none(), "iterator re-executed the statement");
    }

    #[diesel_test_helper::test]
    fn sqlite_iterator_terminates_after_step_error() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        let q = diesel::dsl::sql::<Integer>(
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 5) \
         SELECT CASE WHEN x = 3 THEN json('{') ELSE x END FROM c",
        );
        let items: Vec<Result<i32, String>> = q
            .load_iter::<i32, DefaultLoadingMode>(&mut conn)
            .unwrap()
            .take(100)
            .map(|r| r.map_err(|e| e.to_string()))
            .collect();
        println!("{:?}", &items[..items.len().min(8)]);
        assert_eq!(
            items.len(),
            3,
            "iterator yielded {} items - restarts after error",
            items.len()
        );
        assert_eq!(items[0], Ok(1));
        assert_eq!(items[1], Ok(2));
        assert!(items[2].is_err());
        assert_eq!(items[2].as_ref().unwrap_err().to_string(), "malformed JSON");
    }

    #[diesel_test_helper::test]
    fn held_rows_iterator_terminates_after_step_error() {
        use crate::connection::LoadConnection;
        use crate::row::Row;

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        let q = sql::<Integer>(
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 5) \
               SELECT CASE WHEN x = 3 THEN json('{') ELSE x END FROM c",
        );
        let mut iter = LoadConnection::<DefaultLoadingMode>::load(&mut conn, q).unwrap();

        let first = iter.next().unwrap().unwrap();
        let second = iter.next().unwrap().unwrap();
        let err = iter.next().unwrap().err().expect("step error");
        assert_eq!(err.to_string(), "malformed JSON");
        assert!(iter.next().is_none(), "iterator restarted the statement");
        assert!(iter.next().is_none());

        assert_eq!(first.get_value::<Integer, i32, _>(0).unwrap(), 1);
        assert_eq!(second.get_value::<Integer, i32, _>(0).unwrap(), 2);
    }

    #[diesel_test_helper::test]
    fn held_rows_iterator_continues_after_reborrow_error() {
        use crate::connection::LoadConnection;
        use crate::row::Row;

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute("CREATE TABLE t(x INTEGER); INSERT INTO t VALUES (1), (2), (3);")
            .unwrap();
        let q = sql::<Integer>("SELECT x FROM t");
        let mut iter = LoadConnection::<DefaultLoadingMode>::load(&mut conn, q).unwrap();

        let first = iter.next().unwrap().unwrap();
        let field = first.get(0).unwrap();
        assert!(iter.next().unwrap().is_err());
        drop(field);

        let rest = iter
            .map(|row| row.unwrap().get_value::<Integer, i32, _>(0).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rest, [2, 3]);
    }
}
