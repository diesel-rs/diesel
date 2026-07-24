//! `execute_returning_id` for MySQL inserts, returning the generated
//! `AUTO_INCREMENT` key without an extra round trip.

use super::super::backend::Mysql;
use super::super::connection::MysqlConnection;
use crate::expression::Expression;
use crate::query_builder::{InsertStatement, QueryFragment, QueryId};
use crate::query_source::Table;
use crate::result::QueryResult;

mod sealed {
    /// Sealed marker for the integer SQL types a MySQL `AUTO_INCREMENT` key can
    /// have. `execute_returning_id` is bound on it to reject keys that
    /// `mysql_insert_id` never reports, such as a `Uuid`.
    pub trait AutoIncrementInteger {}
}
use self::sealed::AutoIncrementInteger;

impl AutoIncrementInteger for crate::sql_types::TinyInt {}
impl AutoIncrementInteger for crate::sql_types::SmallInt {}
impl AutoIncrementInteger for crate::sql_types::Integer {}
impl AutoIncrementInteger for crate::sql_types::BigInt {}
impl<T> AutoIncrementInteger for crate::sql_types::Unsigned<T> where T: AutoIncrementInteger {}

impl<T, U, Op, Ret> InsertStatement<T, U, Op, Ret>
where
    T: Table,
    T::PrimaryKey: Expression,
{
    /// Executes this insert and returns the `AUTO_INCREMENT` value MySQL
    /// generated for the new row, read from the client library
    /// (`mysql_insert_id`) with no extra query. This is the MySQL counterpart to
    /// `RETURNING` on the other backends.
    ///
    /// Only inserts into a table with an integer primary key compile, because
    /// `mysql_insert_id` reports only `AUTO_INCREMENT` values and is undefined
    /// for any other key.
    ///
    /// # Caveats
    /// - The value is `0` when the statement generates no `AUTO_INCREMENT` value
    ///   (no such column, or the key was supplied explicitly).
    /// - A multi-row insert returns the first row's id.
    /// - This is the C-API value, which can differ from the SQL
    ///   `LAST_INSERT_ID()` function for `INSERT ... ON DUPLICATE KEY UPDATE` and
    ///   `LAST_INSERT_ID(expr)`.
    ///
    /// # Example
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// let conn = &mut establish_connection();
    /// let new_id: u64 = diesel::insert_into(users)
    ///     .values(name.eq("Ruby"))
    ///     .execute_returning_id(conn)?;
    /// // Two users (ids 1 and 2) are seeded, so Ruby's generated id is 3.
    /// assert_eq!(new_id, 3);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_returning_id(self, conn: &mut MysqlConnection) -> QueryResult<u64>
    where
        Self: QueryFragment<Mysql> + QueryId,
        <T::PrimaryKey as Expression>::SqlType: AutoIncrementInteger,
    {
        conn.execute_returning_id(&self)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::sql_query;

    fn connection() -> MysqlConnection {
        let url = std::env::var("MYSQL_UNIT_TEST_DATABASE_URL")
            .or_else(|_| std::env::var("MYSQL_DATABASE_URL"))
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        MysqlConnection::establish(&url).unwrap()
    }

    crate::table! {
        eri_items (id) {
            id -> Unsigned<BigInt>,
            name -> Text,
        }
    }

    fn create_items_table(conn: &mut MysqlConnection) {
        sql_query(
            "CREATE TEMPORARY TABLE eri_items \
             (id BIGINT UNSIGNED PRIMARY KEY AUTO_INCREMENT, name TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();
    }

    #[diesel_test_helper::test]
    fn execute_returning_id_returns_generated_key() {
        let conn = &mut connection();
        create_items_table(conn);

        let first = crate::insert_into(eri_items::table)
            .values(eri_items::name.eq("a"))
            .execute_returning_id(conn)
            .unwrap();
        assert_eq!(first, 1);

        let second = crate::insert_into(eri_items::table)
            .values(eri_items::name.eq("b"))
            .execute_returning_id(conn)
            .unwrap();
        assert_eq!(second, 2);
    }

    #[diesel_test_helper::test]
    fn execute_returning_id_multi_row_returns_first_id() {
        let conn = &mut connection();
        create_items_table(conn);

        // Three rows get ids 1, 2, 3. MySQL reports the first, not the last.
        let first = crate::insert_into(eri_items::table)
            .values(vec![
                eri_items::name.eq("a"),
                eri_items::name.eq("b"),
                eri_items::name.eq("c"),
            ])
            .execute_returning_id(conn)
            .unwrap();
        assert_eq!(first, 1);
    }
}
