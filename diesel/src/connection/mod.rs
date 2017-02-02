use backend::Backend;
use query_builder::{AsQuery, QueryFragment, QueryId};
use query_source::Queryable;
use result::*;
use types::HasSqlType;

/// To perform simple operations to the backed that implements this trait.
/// `SimpleConnection` will be used for migrations. Every backend shall implement
/// their own way to run migrations scripts.
pub trait SimpleConnection {
    /// Migrations will use this function to run the `up.sql` and `down.sql` files.
    /// New backends shall provide their own mechanism to execute the migration SQL scripts
    fn batch_execute(&self, query: &str) -> QueryResult<()>;
}

/// Every backend shall implement this trait to perform connections to the database.
pub trait Connection: SimpleConnection + Sized {
    /// A trait that provides specific operations for the backed.
    type Backend: Backend;

    /// Establishes a new connection to the database at the given URL. The URL
    /// should be a valid connection string for a given backend. See the
    /// documentation for the specific backend for specifics.
    fn establish(database_url: &str) -> ConnectionResult<Self>;

    /// Executes the given function inside of a database transaction. When
    /// a transaction is already occurring, savepoints will be used to emulate a nested
    /// transaction.
    ///
    /// If the function returns an `Ok`, that value will be returned.  If the
    /// function returns an `Err`,
    /// [`TransactionError::UserReturnedError`](../result/enum.TransactionError.html#variant.UserReturnedError)
    /// will be returned wrapping that value.
    fn transaction<T, E, F>(&self, f: F) -> TransactionResult<T, E> where
        F: FnOnce() -> Result<T, E>,
    {
        try!(self.begin_transaction());
        match f() {
            Ok(value) => {
                try!(self.commit_transaction());
                Ok(value)
            },
            Err(e) => {
                try!(self.rollback_transaction());
                Err(TransactionError::UserReturnedError(e))
            },
        }
    }

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction.
    fn begin_test_transaction(&self) -> QueryResult<()> {
        assert_eq!(self.get_transaction_depth(), 0);
        self.begin_transaction()
    }

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an `Err`.
    fn test_transaction<T, E, F>(&self, f: F) -> T where
        F: FnOnce() -> Result<T, E>,
    {
        let mut user_result = None;
        let _ = self.transaction::<(), _, _>(|| {
            user_result = f().ok();
            Err(())
        });
        user_result.expect("Transaction did not succeed")
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize>;

    #[doc(hidden)]
    fn query_one<T, U>(&self, source: T) -> QueryResult<U> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        self.query_all(source)
            .and_then(|e: Vec<U>| e.into_iter().next().ok_or(Error::NotFound))
    }

    #[doc(hidden)]
    fn query_all<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>;

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend> + QueryId;

    #[doc(hidden)] fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T;
    #[doc(hidden)] fn begin_transaction(&self) -> QueryResult<()>;
    #[doc(hidden)] fn rollback_transaction(&self) -> QueryResult<()>;
    #[doc(hidden)] fn commit_transaction(&self) -> QueryResult<()>;
    #[doc(hidden)] fn get_transaction_depth(&self) -> i32;

    #[doc(hidden)] fn setup_helper_functions(&self);
}
