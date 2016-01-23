extern crate libc;

pub mod pg;

pub use self::pg::PgConnection;

use backend::Backend;
use expression::{AsExpression, Expression, NonAggregate};
use expression::predicates::Eq;
use helper_types::{FindBy, Limit};
use expression::helper_types::AsExpr;
use query_builder::{AsQuery, Query, QueryFragment};
use query_dsl::{FilterDsl, LimitDsl};
use query_source::{Table, Queryable};
use result::*;

#[doc(hidden)]
pub type PrimaryKey<T> = <T as Table>::PrimaryKey;
#[doc(hidden)]
pub type PkType<T> = <PrimaryKey<T> as Expression>::SqlType;
#[doc(hidden)]
pub type FindPredicate<T, PK> = Eq<PrimaryKey<T>, <PK as AsExpression<PkType<T>>>::Expression>;

pub trait SimpleConnection {
    #[doc(hidden)]
    fn batch_execute(&self, query: &str) -> QueryResult<()>;
}

pub trait Connection: SimpleConnection + Sized {
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
    /// [`TransactionError::UserReturnedError`](result/enum.TransactionError.html#variant.UserReturnedError)
    /// will be returned wrapping that value.
    fn transaction<T, E, F>(&self, f: F) -> TransactionResult<T, E> where
        F: FnOnce() -> Result<T, E>;

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction.
    fn begin_test_transaction(&self) -> QueryResult<usize>;

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an `Err`.
    fn test_transaction<T, E, F>(&self, f: F) -> T where
        F: FnOnce() -> Result<T, E>;

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize>;

    #[doc(hidden)]
    fn query_one<T, U>(&self, source: T) -> QueryResult<U> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend>,
        U: Queryable<T::SqlType>;

    #[doc(hidden)]
    fn query_all<'a, T, U: 'a>(&self, source: T) -> QueryResult<Box<Iterator<Item=U> + 'a>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend>,
        U: Queryable<T::SqlType>;

    /// Attempts to find a single record from the given table by primary key.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::result::Error::NotFound;
    /// #     let connection = establish_connection();
    /// let sean = (1, "Sean".to_string());
    /// let tess = (2, "Tess".to_string());
    /// assert_eq!(Ok(sean), connection.find(users, 1));
    /// assert_eq!(Ok(tess), connection.find(users, 2));
    /// assert_eq!(Err::<(i32, String), _>(NotFound), connection.find(users, 3));
    /// # }
    /// ```
    fn find<T, U, PK>(&self, source: T, id: PK) -> QueryResult<U> where
        T: Table + FilterDsl<FindPredicate<T, PK>>,
        FindBy<T, T::PrimaryKey, PK>: LimitDsl,
        Limit<FindBy<T, T::PrimaryKey, PK>>: QueryFragment<Self::Backend>,
        U: Queryable<<Limit<FindBy<T, T::PrimaryKey, PK>> as Query>::SqlType>,
        PK: AsExpression<PkType<T>>,
        AsExpr<PK, T::PrimaryKey>: NonAggregate;

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend>;

    #[doc(hidden)]
    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T;
}
