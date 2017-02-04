use backend::UsesAnsiSavepointSyntax;
use connection::Connection;
use result::QueryResult;

/// Manages the internal transaction state for a connection. You should not
/// interface with this trait unless you are implementing a new connection
/// adapter. You should use [`Connection::transaction`][transaction],
/// [`Connection::test_transaction`][test_transaction], or
/// [`Connection::begin_test_transaction`][begin_test_transaction] instead.
pub trait TransactionManager<Conn: Connection> {
    /// Begin a new transaction. If the transaction depth is greater than 0,
    /// this should create a savepoint instead. This function is expected to
    /// increment the transaction depth by 1.
    fn begin_transaction(&self, conn: &Conn) -> QueryResult<()>;

    /// Rollback the inner-most transcation. If the transaction depth is greater
    /// than 1, this should rollback to the most recent savepoint. This function
    /// is expected to decrement the transaction depth by 1.
    fn rollback_transaction(&self, conn: &Conn) -> QueryResult<()>;

    /// Commit the inner-most transcation. If the transaction depth is greater
    /// than 1, this should release the most recent savepoint. This function is
    /// expected to decrement the transaction depth by 1.
    fn commit_transaction(&self, conn: &Conn) -> QueryResult<()>;

    /// Fetch the current transaction depth. Used to ensure that
    /// `begin_test_transaction` is not called when already inside of a
    /// transaction.
    fn get_transaction_depth(&self) -> u32;
}

use std::cell::Cell;

/// An implementation of `TransactionManager` which can be used for backends
/// which use ANSI standard syntax for savepoints such as SQLite and PostgreSQL.
#[allow(missing_debug_implementations)]
pub struct AnsiTransactionManager {
    transaction_depth: Cell<i32>,
}

impl Default for AnsiTransactionManager {
    fn default() -> Self {
        AnsiTransactionManager {
            transaction_depth: Cell::new(0),
        }
    }
}

impl AnsiTransactionManager {
    pub fn new() -> Self {
        AnsiTransactionManager::default()
    }

    fn change_transaction_depth(&self, by: i32, query: QueryResult<()>) -> QueryResult<()> {
        if query.is_ok() {
            self.transaction_depth.set(self.transaction_depth.get() + by)
        }
        query
    }
}

impl<Conn> TransactionManager<Conn> for AnsiTransactionManager where
    Conn: Connection,
    Conn::Backend: UsesAnsiSavepointSyntax,
{
    fn begin_transaction(&self, conn: &Conn) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(1, if transaction_depth == 0 {
            conn.batch_execute("BEGIN")
        } else {
            conn.batch_execute(&format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
        })
    }

    fn rollback_transaction(&self, conn: &Conn) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth == 1 {
            conn.batch_execute("ROLLBACK")
        } else {
            conn.batch_execute(&format!("ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                                        transaction_depth - 1))
        })
    }

    fn commit_transaction(&self, conn: &Conn) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth <= 1 {
            conn.batch_execute("COMMIT")
        } else {
            conn.batch_execute(&format!("RELEASE SAVEPOINT diesel_savepoint_{}",
                                        transaction_depth - 1))
        })
    }

    fn get_transaction_depth(&self) -> u32 {
        self.transaction_depth.get() as u32
    }
}
