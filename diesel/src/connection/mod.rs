mod cursor;

pub use self::cursor::Cursor;

use db_result::DbResult;
use expression::{AsExpression, Expression, NonAggregate};
use expression::predicates::Eq;
use persistable::{Insertable};
use helper_types::{FindBy, Limit};
use expression::helper_types::AsExpr;
use query_builder::{AsQuery, Query, QueryFragment};
use query_dsl::{FilterDsl, LimitDsl};
use query_source::{Table, Queriable};
use result::*;
use types::{NativeSqlType, ToSql};

#[doc(hidden)]
pub type PrimaryKey<T> = <T as Table>::PrimaryKey;
#[doc(hidden)]
pub type PkType<T> = <PrimaryKey<T> as Expression>::SqlType;
#[doc(hidden)]
pub type FindPredicate<T, PK> = Eq<PrimaryKey<T>, <PK as AsExpression<PkType<T>>>::Expression>;

pub trait Connection: Send + Drop + Sized {

    type DbResult: DbResult;

    #[doc(hidden)]
    fn last_error_message(&self) -> String;

    /// Establishes a new connection to the database at the given URL. The URL
    /// should be a PostgreSQL connection string, as documented at
    /// http://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING
    fn establish(database_url: &str) -> ConnectionResult<Self>;

    /// Executes the given function inside of a database transaction. When
    /// a transaction is already occurring,
    /// [savepoints](http://www.postgresql.org/docs/9.1/static/sql-savepoint.html)
    /// will be used to emulate a nested transaction.
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
    fn batch_execute(&self, query: &str) -> QueryResult<()>; 

    #[doc(hidden)]
    fn query_one<T, U>(&self, source: T) -> QueryResult<U> where
        T: AsQuery,
        U: Queriable<T::SqlType>;

    #[doc(hidden)]
    fn query_all<T, U>(&self, source: T) -> QueryResult<Cursor<T::SqlType, U, Self::DbResult>> where
        T: AsQuery,
        U: Queriable<T::SqlType>;

    #[doc(hidden)]
    fn query_sql<T, U>(&self, query: &str) -> QueryResult<Cursor<T, U, Self::DbResult>> where
        T: NativeSqlType,
        U: Queriable<T>;

    #[doc(hidden)]
    fn query_sql_params<T, U, PT, P>(&self, query: &str, params: &P)
        -> QueryResult<Cursor<T, U, Self::DbResult>> where
        T: NativeSqlType,
        U: Queriable<T>,
        PT: NativeSqlType,
        P: ToSql<PT>;

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
        U: Queriable<<Limit<FindBy<T, T::PrimaryKey, PK>> as Query>::SqlType>,
        PK: AsExpression<PkType<T>>,
        AsExpr<PK, T::PrimaryKey>: NonAggregate;

    #[doc(hidden)]
    fn insert<T, U, Out>(&self, _source: &T, records: U)
        -> QueryResult<Cursor<<T::AllColumns as Expression>::SqlType, Out, Self::DbResult>> where
        T: Table,
        U: Insertable<T>,
        Out: Queriable<<T::AllColumns as Expression>::SqlType>;

    #[doc(hidden)]
    fn insert_returning_count<T, U>(&self, _source: &T, records: U)
        -> QueryResult<usize> where
        T: Table,
        U: Insertable<T>;

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment;

}

