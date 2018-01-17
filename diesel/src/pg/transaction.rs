#![allow(dead_code)]
use backend::Backend;
use connection::TransactionManager;
use pg::Pg;
use prelude::*;
use query_builder::{AstPass, QueryBuilder, QueryFragment};
use result::Error;

/// Used to build a transaction, specifying additional details.
///
/// This struct is returned by [`.build_transaction`].
/// See the documentation for methods on this struct for usage examples.
/// See [the PostgreSQL documentation for `SET TRANSACTION`][pg-docs]
/// for details on the behavior of each option.
///
/// [`.build_transaction`]: struct.PgConnection.html#method.build_transaction
/// [pg-docs]: https://www.postgresql.org/docs/current/static/sql-set-transaction.html
#[allow(missing_debug_implementations)] // False positive. Connection isn't Debug.
#[derive(Clone, Copy)]
#[must_use = "Transaction builder does nothing unless you call `run` on it"]
pub struct TransactionBuilder<'a> {
    connection: &'a PgConnection,
    isolation_level: Option<IsolationLevel>,
    read_mode: Option<ReadMode>,
    deferrable: Option<Deferrable>,
}

impl<'a> TransactionBuilder<'a> {
    pub(crate) fn new(connection: &'a PgConnection) -> Self {
        Self {
            connection,
            isolation_level: None,
            read_mode: None,
            deferrable: None,
        }
    }

    /// Makes the transaction `READ ONLY`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .read_only()
    ///     .run(|| {
    ///         let read_attempt = users.select(name).load::<String>(&conn);
    ///         assert!(read_attempt.is_ok());
    ///
    ///         let write_attempt = diesel::insert_into(users)
    ///             .values(name.eq("Ruby"))
    ///             .execute(&conn);
    ///         assert!(write_attempt.is_err());
    ///
    ///         Ok(())
    ///     })
    /// # }
    /// ```
    pub fn read_only(mut self) -> Self {
        self.read_mode = Some(ReadMode::ReadOnly);
        self
    }

    /// Makes the transaction `READ WRITE`
    ///
    /// This is the default, unless you've changed the
    /// `default_transaction_read_only` configuration parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use diesel::result::Error::RollbackTransaction;
    /// #
    /// # fn main() {
    /// #     assert_eq!(run_test(), Err(RollbackTransaction));
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .read_write()
    ///     .run(|| {
    ///         let read_attempt = users.select(name).load::<String>(&conn);
    ///         assert!(read_attempt.is_ok());
    ///
    ///         let write_attempt = diesel::insert_into(users)
    ///             .values(name.eq("Ruby"))
    ///             .execute(&conn);
    ///         assert!(write_attempt.is_ok());
    ///
    /// #       Err(RollbackTransaction)
    /// #       /*
    ///         Ok(())
    /// #       */
    ///     })
    /// # }
    /// ```
    pub fn read_write(mut self) -> Self {
        self.read_mode = Some(ReadMode::ReadWrite);
        self
    }

    /// Makes the transaction `DEFERRABLE`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .deferrable()
    ///     .run(|| Ok(()))
    /// # }
    /// ```
    pub fn deferrable(mut self) -> Self {
        self.deferrable = Some(Deferrable::Deferrable);
        self
    }

    /// Makes the transaction `NOT DEFERRABLE`
    ///
    /// This is the default, unless you've changed the
    /// `default_transaction_deferrable` configuration parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .not_deferrable()
    ///     .run(|| Ok(()))
    /// # }
    /// ```
    pub fn not_deferrable(mut self) -> Self {
        self.deferrable = Some(Deferrable::NotDeferrable);
        self
    }

    /// Makes the transaction `ISOLATION LEVEL READ COMMITTED`
    ///
    /// This is the default, unless you've changed the
    /// `default_transaction_isolation_level` configuration parameter.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .read_committed()
    ///     .run(|| Ok(()))
    /// # }
    /// ```
    pub fn read_committed(mut self) -> Self {
        self.isolation_level = Some(IsolationLevel::ReadCommitted);
        self
    }

    /// Makes the transaction `ISOLATION LEVEL REPEATABLE READ`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .repeatable_read()
    ///     .run(|| Ok(()))
    /// # }
    /// ```
    pub fn repeatable_read(mut self) -> Self {
        self.isolation_level = Some(IsolationLevel::RepeatableRead);
        self
    }

    /// Makes the transaction `ISOLATION LEVEL SERIALIZABLE`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = connection_no_transaction();
    /// conn.build_transaction()
    ///     .serializable()
    ///     .run(|| Ok(()))
    /// # }
    /// ```
    pub fn serializable(mut self) -> Self {
        self.isolation_level = Some(IsolationLevel::Serializable);
        self
    }

    /// Runs the given function inside of the transaction
    /// with the parameters given to this builder.
    ///
    /// Returns an error if the connection is already inside a transaction.
    pub fn run<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<Error>,
    {
        let mut query_builder = <Pg as Backend>::QueryBuilder::default();
        self.to_sql(&mut query_builder)?;
        let sql = query_builder.finish();
        let transaction_manager = self.connection.transaction_manager();

        transaction_manager.begin_transaction_sql(self.connection, &sql)?;
        match f() {
            Ok(value) => {
                transaction_manager.commit_transaction(self.connection)?;
                Ok(value)
            }
            Err(e) => {
                transaction_manager.rollback_transaction(self.connection)?;
                Err(e)
            }
        }
    }
}

impl<'a> QueryFragment<Pg> for TransactionBuilder<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("BEGIN TRANSACTION");
        if let Some(ref isolation_level) = self.isolation_level {
            isolation_level.walk_ast(out.reborrow())?;
        }
        if let Some(ref read_mode) = self.read_mode {
            read_mode.walk_ast(out.reborrow())?;
        }
        if let Some(ref deferrable) = self.deferrable {
            deferrable.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum IsolationLevel {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl QueryFragment<Pg> for IsolationLevel {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" ISOLATION LEVEL ");
        match *self {
            IsolationLevel::ReadCommitted => out.push_sql("READ COMMITTED"),
            IsolationLevel::RepeatableRead => out.push_sql("REPEATABLE READ"),
            IsolationLevel::Serializable => out.push_sql("SERIALIZABLE"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ReadMode {
    ReadOnly,
    ReadWrite,
}

impl QueryFragment<Pg> for ReadMode {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        match *self {
            ReadMode::ReadOnly => out.push_sql(" READ ONLY"),
            ReadMode::ReadWrite => out.push_sql(" READ WRITE"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Deferrable {
    Deferrable,
    NotDeferrable,
}

impl QueryFragment<Pg> for Deferrable {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        match *self {
            Deferrable::Deferrable => out.push_sql(" DEFERRABLE"),
            Deferrable::NotDeferrable => out.push_sql(" NOT DEFERRABLE"),
        }
        Ok(())
    }
}

#[test]
fn test_transaction_builder_generates_correct_sql() {
    extern crate dotenv;

    macro_rules! assert_sql {
        ($query:expr, $sql:expr) => {
            let mut query_builder = <Pg as Backend>::QueryBuilder::default();
            $query.to_sql(&mut query_builder).unwrap();
            let sql = query_builder.finish();
            assert_eq!(sql, $sql);
        }
    }

    let database_url = dotenv::var("PG_DATABASE_URL")
        .or_else(|_| dotenv::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let conn = PgConnection::establish(&database_url).unwrap();

    let t = conn.build_transaction();
    assert_sql!(t, "BEGIN TRANSACTION");
    assert_sql!(t.read_only(), "BEGIN TRANSACTION READ ONLY");
    assert_sql!(t.read_write(), "BEGIN TRANSACTION READ WRITE");
    assert_sql!(t.deferrable(), "BEGIN TRANSACTION DEFERRABLE");
    assert_sql!(t.not_deferrable(), "BEGIN TRANSACTION NOT DEFERRABLE");
    assert_sql!(
        t.read_committed(),
        "BEGIN TRANSACTION ISOLATION LEVEL READ COMMITTED"
    );
    assert_sql!(
        t.repeatable_read(),
        "BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ"
    );
    assert_sql!(
        t.serializable(),
        "BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE"
    );
    assert_sql!(
        t.serializable().deferrable().read_only(),
        "BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY DEFERRABLE"
    );
}
