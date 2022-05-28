//! Types related to database connections

mod statement_cache;
mod transaction_manager;

use std::fmt::Debug;

use crate::backend::Backend;
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::query_dsl::load_dsl::CompatibleType;
use crate::result::*;

#[doc(hidden)]
pub use self::statement_cache::{MaybeCached, StatementCache, StatementCacheKey};
pub use self::transaction_manager::{AnsiTransactionManager, TransactionManager};

/// Perform simple operations on a backend.
///
/// You should likely use [`Connection`] instead.
pub trait SimpleConnection {
    /// Execute multiple SQL statements within the same string.
    ///
    /// This function is used to execute migrations,
    /// which may contain more than one SQL statement.
    fn batch_execute(&mut self, query: &str) -> QueryResult<()>;
}

/// A connection to a database
pub trait Connection: SimpleConnection + Sized + Send {
    /// The backend this type connects to
    type Backend: Backend;

    #[doc(hidden)]
    type TransactionManager: TransactionManager<Self>;

    /// Establishes a new connection to the database
    ///
    /// The argument to this method and the method's behavior varies by backend.
    /// See the documentation for that backend's connection class
    /// for details about what it accepts and how it behaves.
    fn establish(database_url: &str) -> ConnectionResult<Self>;

    /// Executes the given function inside of a database transaction
    ///
    /// If there is already an open transaction,
    /// savepoints will be used instead.
    ///
    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted. If the rollback succeeds,
    /// the original error will be returned, otherwise the error generated by the rollback
    /// will be wrapped into an `Error::RollbackError` and returned. In the second case
    /// the connection should be considered broken as it contains a uncommitted unabortable
    /// open transaction.
    ///
    /// If a nested transaction fails to release the corresponding savepoint
    /// a rollback will be attempted. If the rollback succeeds,
    /// the original error will be returned, otherwise the error generated by the rollback
    /// will be wrapped into an `Error::RollbackError` and returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// conn.transaction::<_, Error, _>(|conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// })?;
    ///
    /// conn.transaction::<(), _, _>(|conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Pascal"))
    ///         .execute(conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby", "Pascal"], all_names);
    ///
    ///     // If we want to roll back the transaction, but don't have an
    ///     // actual error to return, we can return `RollbackTransaction`.
    ///     Err(Error::RollbackTransaction)
    /// });
    ///
    /// let all_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<Error>,
    {
        Self::TransactionManager::begin_transaction(self)?;
        match f(&mut *self) {
            Ok(value) => {
                Self::TransactionManager::commit_transaction(self)?;
                Ok(value)
            }
            Err(e) => {
                Self::TransactionManager::rollback_transaction(self)
                    .map_err(|e| Error::RollbackError(Box::new(e)))?;
                Err(e)
            }
        }
    }

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction.
    fn begin_test_transaction(&mut self) -> QueryResult<()> {
        assert_eq!(Self::TransactionManager::get_transaction_depth(self), 0);
        Self::TransactionManager::begin_transaction(self)
    }

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// conn.test_transaction::<_, Error, _>(|conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// });
    ///
    /// // Even though we returned `Ok`, the transaction wasn't committed.
    /// let all_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(vec!["Sean", "Tess"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn test_transaction<T, E, F>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: Debug,
    {
        let mut user_result = None;
        let _ = self.transaction::<(), _, _>(|conn| {
            user_result = f(conn).ok();
            Err(Error::RollbackTransaction)
        });
        user_result.expect("Transaction did not succeed")
    }

    #[doc(hidden)]
    fn execute(&mut self, query: &str) -> QueryResult<usize>;

    #[doc(hidden)]
    fn load<T, U, ST>(&mut self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        T::SqlType: CompatibleType<U, Self::Backend, SqlType = ST>,
        U: FromSqlRow<ST, Self::Backend>,
        Self::Backend: QueryMetadata<T::SqlType>;

    #[doc(hidden)]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId;

    #[doc(hidden)]
    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData;
}

/// A variant of the [`Connection`](trait.Connection.html) trait that is
/// usable with dynamic dispatch
///
/// If you are looking for a way to use pass database connections
/// for different database backends around in your application
/// this trait won't help you much. Normally you should only
/// need to use this trait if you are interacting with a connection
/// passed to a [`Migration`](../migration/trait.Migration.html)
pub trait BoxableConnection<DB: Backend>: SimpleConnection + std::any::Any{
    #[doc(hidden)]
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<C> BoxableConnection<C::Backend> for C
where
    C: Connection + std::any::Any,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl<DB: 'static + Backend> dyn BoxableConnection<DB> {
    /// Downcast the current connection to a specific connection
    /// type.
    ///
    /// This will return `None` if the underlying
    /// connection does not match the corresponding
    /// type, otherwise a reference to the underlying connection is returned
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Connection<Backend = DB> + 'static,
    {
        self.as_any().downcast_ref::<T>()
    }

    /// Check if the current connection is
    /// a specific connection type
    pub fn is<T>(&self) -> bool
    where
        T: Connection<Backend = DB> + 'static,
    {
        self.as_any().is::<T>()
    }
}
