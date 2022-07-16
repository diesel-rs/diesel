use super::raw::RawConnection;
use super::result::PgResult;
use super::row::PgRow;
use std::rc::Rc;

#[allow(missing_debug_implementations)]
pub struct Cursor {
    current_row: usize,
    db_result: Rc<PgResult>,
}

impl Cursor {
    pub(super) fn new(result: PgResult, conn: &mut RawConnection) -> crate::QueryResult<Cursor> {
        let next_res = conn.get_next_result()?;
        debug_assert!(next_res.is_none());
        Ok(Self {
            current_row: 0,
            db_result: Rc::new(result),
        })
    }
}

impl ExactSizeIterator for Cursor {
    fn len(&self) -> usize {
        self.db_result.num_rows() - self.current_row
    }
}

impl Iterator for Cursor {
    type Item = crate::QueryResult<PgRow>;

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

/// The type returned by various [`Connection`] methods.
/// Acts as an iterator over `T`.
#[allow(missing_debug_implementations)]
pub struct RowByRowCursor<'a> {
    current_row: usize,
    db_result: Rc<PgResult>,
    conn: &'a mut super::ConnectionAndTransactionManager,
}

impl<'a> RowByRowCursor<'a> {
    pub(super) fn new(
        db_result: PgResult,
        conn: &'a mut super::ConnectionAndTransactionManager,
    ) -> Self {
        RowByRowCursor {
            current_row: 0,
            db_result: Rc::new(db_result),
            conn,
        }
    }
}

impl Iterator for RowByRowCursor<'_> {
    type Item = crate::QueryResult<PgRow>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row > 0 {
            let get_next_result = super::update_transaction_manager_status(
                self.conn.raw_connection.get_next_result(),
                self.conn,
            );
            match get_next_result {
                Ok(Some(res)) => {
                    if let Some(old_res) = Rc::get_mut(&mut self.db_result) {
                        *old_res = res;
                    } else {
                        self.db_result = Rc::new(res);
                    }
                    self.current_row = 0;
                }
                Ok(None) => {
                    return None;
                }
                Err(e) => return Some(Err(e)),
            }
        }
        if self.current_row < self.db_result.num_rows() {
            let row = self.db_result.clone().get_row(self.current_row);
            self.current_row += 1;
            Some(Ok(row))
        } else {
            None
        }
    }
}

impl Drop for RowByRowCursor<'_> {
    fn drop(&mut self) {
        loop {
            let res = super::update_transaction_manager_status(
                self.conn.raw_connection.get_next_result(),
                self.conn,
            );
            if matches!(res, Err(_) | Ok(None)) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::DefaultLoadingMode;
    use crate::pg::PgRowByRowLoadingMode;

    #[test]
    fn fun_with_row_iters() {
        crate::table! {
            #[allow(unused_parens)]
            users(id) {
                id -> Integer,
                name -> Text,
            }
        }

        use crate::connection::LoadConnection;
        use crate::deserialize::{FromSql, FromSqlRow};
        use crate::pg::Pg;
        use crate::prelude::*;
        use crate::row::{Field, Row};
        use crate::sql_types;

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(vec![
                (users::id.eq(1), users::name.eq("Sean")),
                (users::id.eq(2), users::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();

        let query = users::table.select((users::id, users::name));

        let expected = vec![(1, String::from("Sean")), (2, String::from("Tess"))];

        let row_iter = LoadConnection::<DefaultLoadingMode>::load(conn, &query).unwrap();
        for (row, expected) in row_iter.zip(&expected) {
            let row = row.unwrap();

            let deserialized = <(i32, String) as FromSqlRow<
                (sql_types::Integer, sql_types::Text),
                _,
            >>::build_from_row(&row)
            .unwrap();

            assert_eq!(&deserialized, expected);
        }

        {
            let collected_rows = LoadConnection::<DefaultLoadingMode>::load(conn, &query)
                .unwrap()
                .collect::<Vec<_>>();

            for (row, expected) in collected_rows.iter().zip(&expected) {
                let deserialized = row
                    .as_ref()
                    .map(|row| {
                        <(i32, String) as FromSqlRow<
                                (sql_types::Integer, sql_types::Text),
                            _,
                            >>::build_from_row(row).unwrap()
                    })
                    .unwrap();

                assert_eq!(&deserialized, expected);
            }
        }

        let mut row_iter = LoadConnection::<DefaultLoadingMode>::load(conn, &query).unwrap();

        let first_row = row_iter.next().unwrap().unwrap();
        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        let second_row = row_iter.next().unwrap().unwrap();
        let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());
        let second_values = (second_fields.0.value(), second_fields.1.value());

        assert!(row_iter.next().is_none());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(second_values.0).unwrap(),
            expected[1].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(second_values.1).unwrap(),
            expected[1].1
        );

        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );
    }

    #[test]
    fn loading_modes_return_the_same_result() {
        use crate::prelude::*;

        crate::table! {
            #[allow(unused_parens)]
            users(id) {
                id -> Integer,
                name -> Text,
            }
        }

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(vec![
                (users::id.eq(1), users::name.eq("Sean")),
                (users::id.eq(2), users::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();

        let users_by_default_mode = users::table
            .select(users::name)
            .load_iter::<String, DefaultLoadingMode>(conn)
            .unwrap()
            .collect::<QueryResult<Vec<_>>>()
            .unwrap();
        let users_row_by_row = users::table
            .select(users::name)
            .load_iter::<String, PgRowByRowLoadingMode>(conn)
            .unwrap()
            .collect::<QueryResult<Vec<_>>>()
            .unwrap();
        assert_eq!(users_by_default_mode, users_row_by_row);
        assert_eq!(users_by_default_mode, vec!["Sean", "Tess"]);
    }

    #[test]
    fn fun_with_row_iters_row_by_row() {
        crate::table! {
            #[allow(unused_parens)]
            users(id) {
                id -> Integer,
                name -> Text,
            }
        }

        use crate::connection::LoadConnection;
        use crate::deserialize::{FromSql, FromSqlRow};
        use crate::pg::Pg;
        use crate::prelude::*;
        use crate::row::{Field, Row};
        use crate::sql_types;

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(vec![
                (users::id.eq(1), users::name.eq("Sean")),
                (users::id.eq(2), users::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();

        let query = users::table.select((users::id, users::name));

        let expected = vec![(1, String::from("Sean")), (2, String::from("Tess"))];

        let row_iter = LoadConnection::<PgRowByRowLoadingMode>::load(conn, &query).unwrap();
        for (row, expected) in row_iter.zip(&expected) {
            let row = row.unwrap();

            let deserialized = <(i32, String) as FromSqlRow<
                (sql_types::Integer, sql_types::Text),
                _,
            >>::build_from_row(&row)
            .unwrap();

            assert_eq!(&deserialized, expected);
        }

        {
            let collected_rows = LoadConnection::<PgRowByRowLoadingMode>::load(conn, &query)
                .unwrap()
                .collect::<Vec<_>>();

            for (row, expected) in collected_rows.iter().zip(&expected) {
                let deserialized = row
                    .as_ref()
                    .map(|row| {
                        <(i32, String) as FromSqlRow<
                                (sql_types::Integer, sql_types::Text),
                            _,
                            >>::build_from_row(row).unwrap()
                    })
                    .unwrap();

                assert_eq!(&deserialized, expected);
            }
        }

        let mut row_iter = LoadConnection::<PgRowByRowLoadingMode>::load(conn, &query).unwrap();

        let first_row = row_iter.next().unwrap().unwrap();
        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        let second_row = row_iter.next().unwrap().unwrap();
        let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());
        let second_values = (second_fields.0.value(), second_fields.1.value());

        assert!(row_iter.next().is_none());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(second_values.0).unwrap(),
            expected[1].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(second_values.1).unwrap(),
            expected[1].1
        );

        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );
    }
}
