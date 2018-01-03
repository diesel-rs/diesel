///! Types related to database connections

mod statement_cache;
mod transaction_manager;

use std::fmt::Debug;

use backend::Backend;
use query_builder::{AsQuery, QueryFragment, QueryId};
use query_source::{Queryable, QueryableByName};
use result::*;
use types::HasSqlType;

pub use self::transaction_manager::{AnsiTransactionManager, TransactionManager};
#[doc(hidden)]
pub use self::statement_cache::{MaybeCached, StatementCache, StatementCacheKey};

/// Perform simple operations on a backend.
///
/// You should likely use [`Connection`](trait.Connection.html) instead.
pub trait SimpleConnection {
    /// Execute multiple SQL statements within the same string.
    ///
    /// This function is used to execute migrations,
    /// which may contain more than one SQL statement.
    fn batch_execute(&self, query: &str) -> QueryResult<()>;
}

/// A connection to a database
pub trait Connection: SimpleConnection + Sized + Send {
    /// The backend this type connects to
    type Backend: Backend;
    #[doc(hidden)]
    type TransactionManager: TransactionManager<Self>;

    /// Establishes a new connection to the database
    ///
    /// The argument to this method varies by backend.
    /// See the documentation for that backend's connection class
    /// for details about what it accepts.
    fn establish(database_url: &str) -> ConnectionResult<Self>;

    /// Executes the given function inside of a database transaction
    ///
    /// If there is already an open transaction,
    /// savepoints will be used instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = establish_connection();
    /// conn.transaction::<_, Error, _>(|| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(&conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(&conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// })?;
    ///
    /// conn.transaction::<(), _, _>(|| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Pascal"))
    ///         .execute(&conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(&conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby", "Pascal"], all_names);
    ///
    ///     // If we want to roll back the transaction, but don't have an
    ///     // actual error to return, we can return `RollbackTransaction`.
    ///     Err(Error::RollbackTransaction)
    /// });
    ///
    /// let all_names = users.select(name).load::<String>(&conn)?;
    /// assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<Error>,
    {
        let transaction_manager = self.transaction_manager();
        try!(transaction_manager.begin_transaction(self));
        match f() {
            Ok(value) => {
                try!(transaction_manager.commit_transaction(self));
                Ok(value)
            }
            Err(e) => {
                try!(transaction_manager.rollback_transaction(self));
                Err(e)
            }
        }
    }

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction.
    fn begin_test_transaction(&self) -> QueryResult<()> {
        let transaction_manager = self.transaction_manager();
        assert_eq!(transaction_manager.get_transaction_depth(), 0);
        transaction_manager.begin_transaction(self)
    }

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = establish_connection();
    /// conn.test_transaction::<_, Error, _>(|| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(&conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(&conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// });
    ///
    /// // Even though we returned `Ok`, the transaction wasn't committed.
    /// let all_names = users.select(name).load::<String>(&conn)?;
    /// assert_eq!(vec!["Sean", "Tess"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn test_transaction<T, E, F>(&self, f: F) -> T
    where
        F: FnOnce() -> Result<T, E>,
        E: Debug,
    {
        let mut user_result = None;
        let _ = self.transaction::<(), _, _>(|| {
            user_result = f().ok();
            Err(Error::RollbackTransaction)
        });
        user_result.expect("Transaction did not succeed")
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize>;

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>;

    #[doc(hidden)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Self::Backend> + QueryId,
        U: QueryableByName<Self::Backend>;

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId;

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager;
}
