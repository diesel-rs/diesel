use crate::backend::UsesAnsiSavepointSyntax;
use crate::connection::{Connection, SimpleConnection};
use crate::result::QueryResult;

/// Manages the internal transaction state for a connection.
///
/// You will not need to interact with this trait, unless you are writing an
/// implementation of [`Connection`](trait.Connection.html).
pub trait TransactionManager<Conn: Connection> {
    /// Begin a new transaction or savepoint
    ///
    /// If the transaction depth is greater than 0,
    /// this should create a savepoint instead.
    /// This function is expected to increment the transaction depth by 1.
    fn begin_transaction(&self, conn: &Conn) -> QueryResult<()>;

    /// Rollback the inner-most transaction or savepoint
    ///
    /// If the transaction depth is greater than 1,
    /// this should rollback to the most recent savepoint.
    /// This function is expected to decrement the transaction depth by 1.
    fn rollback_transaction(&self, conn: &Conn) -> QueryResult<()>;

    /// Commit the inner-most transaction or savepoint
    ///
    /// If the transaction depth is greater than 1,
    /// this should release the most recent savepoint.
    /// This function is expected to decrement the transaction depth by 1.
    fn commit_transaction(&self, conn: &Conn) -> QueryResult<()>;

    /// Fetch the current transaction depth
    ///
    /// Used to ensure that `begin_test_transaction` is not called when already
    /// inside of a transaction.
    fn get_transaction_depth(&self) -> u32;
}

use std::cell::Cell;

/// An implementation of `TransactionManager` which can be used for backends
/// which use ANSI standard syntax for savepoints such as SQLite and PostgreSQL.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct AnsiTransactionManager {
    transaction_depth: Cell<i32>,
}

impl AnsiTransactionManager {
    /// Create a new transaction manager
    pub fn new() -> Self {
        AnsiTransactionManager::default()
    }

    fn change_transaction_depth(&self, by: i32, query: QueryResult<()>) -> QueryResult<()> {
        if query.is_ok() {
            self.transaction_depth
                .set(self.transaction_depth.get() + by)
        }
        query
    }

    /// Begin a transaction with custom SQL
    ///
    /// This is used by connections to implement more complex transaction APIs
    /// to set things such as isolation levels.
    /// Returns an error if already inside of a transaction.
    pub fn begin_transaction_sql<Conn>(&self, conn: &Conn, sql: &str) -> QueryResult<()>
    where
        Conn: SimpleConnection,
    {
        use crate::result::Error::AlreadyInTransaction;

        if self.transaction_depth.get() == 0 {
            self.change_transaction_depth(1, conn.batch_execute(sql))
        } else {
            Err(AlreadyInTransaction)
        }
    }
}

impl<Conn> TransactionManager<Conn> for AnsiTransactionManager
where
    Conn: Connection,
    Conn::Backend: UsesAnsiSavepointSyntax,
{
    fn begin_transaction(&self, conn: &Conn) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(
            1,
            if transaction_depth == 0 {
                conn.batch_execute("BEGIN")
            } else {
                conn.batch_execute(&format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
            },
        )
    }

    fn rollback_transaction(&self, conn: &Conn) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(
            -1,
            if transaction_depth == 1 {
                conn.batch_execute("ROLLBACK")
            } else {
                conn.batch_execute(&format!(
                    "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                    transaction_depth - 1
                ))
            },
        )
    }

    fn commit_transaction(&self, conn: &Conn) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(
            -1,
            if transaction_depth <= 1 {
                conn.batch_execute("COMMIT")
            } else {
                conn.batch_execute(&format!(
                    "RELEASE SAVEPOINT diesel_savepoint_{}",
                    transaction_depth - 1
                ))
            },
        )
    }

    fn get_transaction_depth(&self) -> u32 {
        self.transaction_depth.get() as u32
    }
}
