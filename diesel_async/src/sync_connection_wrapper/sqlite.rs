use diesel::connection::AnsiTransactionManager;
use diesel::SqliteConnection;

use crate::sync_connection_wrapper::SyncTransactionManagerWrapper;
use crate::transaction_manager::AsyncFunc;
use crate::TransactionManager;

use super::SyncConnectionWrapper;

impl SyncConnectionWrapper<SqliteConnection> {
    /// Run a transaction with `BEGIN IMMEDIATE`
    ///
    /// This method will return an error if a transaction is already open.
    ///
    /// **WARNING:** Canceling the returned future does currently **not**
    /// close an already open transaction. You may end up with a connection
    /// containing a dangling transaction.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    /// use diesel_async::{RunQueryDsl, AsyncConnection};
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// #     run_test().await.unwrap();
    /// # }
    /// #
    /// # async fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut connection_no_transaction().await;
    /// conn.immediate_transaction(async |conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)
    ///         .await?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn).await?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// }).await
    /// # }
    /// ```
    pub async fn immediate_transaction<'a, R, E, F>(&mut self, f: F) -> Result<R, E>
    where
        for<'r> F: AsyncFnOnce(&'r mut Self) -> Result<R, E>
            + AsyncFunc<&'r mut Self, Result<R, E>, Fut: Send>
            + Send
            + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
    {
        self.transaction_sql(f, "BEGIN IMMEDIATE").await
    }

    /// Run a transaction with `BEGIN EXCLUSIVE`
    ///
    /// This method will return an error if a transaction is already open.
    ///
    /// **WARNING:** Canceling the returned future does currently **not**
    /// close an already open transaction. You may end up with a connection
    /// containing a dangling transaction.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    /// use diesel_async::{RunQueryDsl, AsyncConnection};
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// #     run_test().await.unwrap();
    /// # }
    /// #
    /// # async fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut connection_no_transaction().await;
    /// conn.exclusive_transaction(async |conn|  {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)
    ///         .await?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn).await?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// }).await
    /// # }
    /// ```
    pub async fn exclusive_transaction<'a, R, E, F>(&mut self, f: F) -> Result<R, E>
    where
        for<'r> F: AsyncFnOnce(&'r mut Self) -> Result<R, E>
            + AsyncFunc<&'r mut Self, Result<R, E>, Fut: Send>
            + Send
            + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
    {
        self.transaction_sql(f, "BEGIN EXCLUSIVE").await
    }

    async fn transaction_sql<'a, R, E, F>(&mut self, f: F, sql: &'static str) -> Result<R, E>
    where
        for<'r> F: AsyncFnOnce(&'r mut Self) -> Result<R, E>
            + AsyncFunc<&'r mut Self, Result<R, E>, Fut: Send>
            + Send
            + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
    {
        self.spawn_blocking(|conn| AnsiTransactionManager::begin_transaction_sql(conn, sql))
            .await?;

        match f(&mut *self).await {
            Ok(value) => {
                SyncTransactionManagerWrapper::<AnsiTransactionManager>::commit_transaction(
                    &mut *self,
                )
                .await?;
                Ok(value)
            }
            Err(e) => {
                SyncTransactionManagerWrapper::<AnsiTransactionManager>::rollback_transaction(
                    &mut *self,
                )
                .await?;
                Err(e)
            }
        }
    }
}
