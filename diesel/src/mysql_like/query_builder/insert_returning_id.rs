//! `execute_returning_id` for MySQL and MariaDB inserts.

use core::num::NonZeroU64;

use crate::mysql_like::MysqlLikeBackend;
use crate::mysql_like::connection::MysqlLikeConnection;
use crate::query_builder::{InsertStatement, QueryFragment, QueryId, SingleRowInsertValues};
use crate::query_source::{AutoIncrementTable, QuerySource};
use crate::result::QueryResult;

impl<T: QuerySource, U, Op, Ret> InsertStatement<T, U, Op, Ret> {
    /// Executes this insert and returns the `AUTO_INCREMENT` value it set, or
    /// `None` if it set none. The value comes from [`mysql_stmt_insert_id`],
    /// read straight from the executed statement, so it needs neither a
    /// `RETURNING` clause nor an extra `SELECT LAST_INSERT_ID()` round trip.
    ///
    /// Only available for single-row inserts into a table whose
    /// [`table!`](crate::table!) definition marks a column with
    /// `#[auto_increment]` (emitted by `diesel print-schema`).
    ///
    /// [`mysql_stmt_insert_id`]: https://dev.mysql.com/doc/c-api/8.4/en/mysql-stmt-insert-id.html
    ///
    /// # Caveats
    /// - A key you supply explicitly is reported back rather than `None`, as
    ///   is the key of the row an `ON DUPLICATE KEY UPDATE` clause updates
    /// - An `INSERT IGNORE` whose row is skipped gives `None`
    /// - Differs from the SQL `LAST_INSERT_ID()` function in the cases above
    ///   and for `LAST_INSERT_ID(expr)`
    ///
    /// # Example
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// use core::num::NonZeroU64;
    /// let conn = &mut establish_connection();
    /// let new_id = diesel::insert_into(users)
    ///     .values(name.eq("Ruby"))
    ///     .execute_returning_id(conn)?;
    /// // Ids 1 and 2 are seeded, so the generated id is 3.
    /// assert_eq!(new_id, NonZeroU64::new(3));
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_returning_id<DB>(
        self,
        conn: &mut MysqlLikeConnection<DB>,
    ) -> QueryResult<Option<NonZeroU64>>
    where
        DB: MysqlLikeBackend,
        T: AutoIncrementTable,
        U: SingleRowInsertValues<T>,
        Self: QueryFragment<DB> + QueryId,
    {
        conn.execute_returning_id(&self)
    }
}
