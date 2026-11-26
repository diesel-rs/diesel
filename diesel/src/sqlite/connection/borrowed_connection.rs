use super::{RawConnection, SqliteConnection};
use crate::connection::instrumentation::DynInstrumentation;
use crate::connection::statement_cache::StatementCache;
use crate::connection::*;
use crate::expression::QueryMetadata;
use crate::prelude::{ConnectionResult, QueryResult};
use crate::query_builder::{Query, QueryFragment, QueryId};
use crate::sqlite::Sqlite;
use core::mem::ManuallyDrop;

/// A borrowed SQLite connection
///
/// This type is provided to various hook functions called by SQLite internally
/// It mostly acts as `&mut SqliteConnection` with the additional constraint
/// that you cannot move out/replace the inner connection handle
///
/// Any hook (update, commit, rollback, progress, wal, busy handler, authorizer, trace, collation needed)
/// is reset after the closure that gets this connection type as argument ends. This specifically means
/// that any hook registered during [`register_auto_extension`] won't trigger later
#[allow(missing_debug_implementations)]
pub struct BorrowedSqliteConnection {
    // SAFETY invariant: never move the inner connection
    // out of the container. we rely on the fact that we never
    // run the destructor for the connection as it's only
    // borrowed from an outer connection and not actually
    // owned
    inner: ManuallyDrop<SqliteConnection>,
}

impl Drop for BorrowedSqliteConnection {
    #[allow(unsafe_code)] // manual memory management
    fn drop(&mut self) {
        // SAFETY: `self.inner` is not touched again after this take.
        let conn = unsafe { core::mem::ManuallyDrop::take(&mut self.inner) };
        let SqliteConnection {
            statement_cache,
            raw_connection,
            transaction_state: _,
            metadata_lookup: (),
            instrumentation,
        } = conn;
        // wrap the inner connection into manually drop again
        // to make sure that we don't call the destructor under any condition (even panics)
        let mut raw_connection = core::mem::ManuallyDrop::new(raw_connection);
        // we need to make sure to unregister all the hooks here to not leak memory
        raw_connection.reset_registered_hooks();
        // Finalize prepared statements, but do not run `RawConnection`'s
        // `Drop`, which would close a handle we do not own.
        drop(statement_cache);
        drop(instrumentation);
    }
}

impl SimpleConnection for BorrowedSqliteConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        SqliteConnection::batch_execute(&mut self.inner, query)
    }
}

impl Connection for BorrowedSqliteConnection {
    type Backend = Sqlite;

    type TransactionManager = AnsiTransactionManager;

    fn establish(_database_url: &str) -> ConnectionResult<Self> {
        Err(crate::ConnectionError::BadConnection(
            "Cannot create a pinned sqlite connection".into(),
        ))
    }

    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        SqliteConnection::execute_returning_count(&mut self.inner, source)
    }

    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData {
        SqliteConnection::transaction_state(&mut self.inner)
    }

    fn instrumentation(&mut self) -> &mut dyn Instrumentation {
        SqliteConnection::instrumentation(&mut self.inner)
    }

    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
        SqliteConnection::set_instrumentation(&mut self.inner, instrumentation);
    }

    fn set_prepared_statement_cache_size(&mut self, size: CacheSize) {
        SqliteConnection::set_prepared_statement_cache_size(&mut self.inner, size)
    }
}

impl LoadConnection for BorrowedSqliteConnection {
    type Cursor<'conn, 'query>
        = <SqliteConnection as LoadConnection>::Cursor<'conn, 'query>
    where
        Self: 'conn;

    type Row<'conn, 'query>
        = <SqliteConnection as LoadConnection>::Row<'conn, 'query>
    where
        Self: 'conn;

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        SqliteConnection::load(&mut self.inner, source)
    }
}

impl ConnectionSealed for BorrowedSqliteConnection {}

impl BorrowedSqliteConnection {
    pub(super) fn new(conn: RawConnection) -> Self {
        BorrowedSqliteConnection {
            inner: core::mem::ManuallyDrop::new(SqliteConnection {
                statement_cache: StatementCache::new(),
                raw_connection: conn,
                transaction_state: AnsiTransactionManager::default(),
                metadata_lookup: (),
                instrumentation: DynInstrumentation::default_instrumentation(),
            }),
        }
    }

    /// Get access to the underlying SqliteConnection
    ///
    /// # Safety
    ///
    /// - You are not allowed to move the connection out of the provided reference or otherwise
    ///   replace the connection
    #[allow(unsafe_code)] // helper to allow others accessing the connection
    pub unsafe fn inner_mut(&mut self) -> &mut SqliteConnection {
        &mut self.inner
    }
}

// Dropping the borrowed connection must not unregister hooks of the outer
// connection that shares the same `sqlite3` handle.
//
// These tests rely on sqlite calling the registered hooks, which is not
// supported when running under miri with a native libsqlite3.
#[cfg(all(
    test,
    not(any(miri, all(target_family = "wasm", target_os = "unknown")))
))]
mod tests {
    use super::*;
    use crate::connection::SimpleConnection;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    // `tempfile::tempdir()` panics on WASM due to the lack of a filesystem,
    // and WAL mode does not work with `:memory:`.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_hook_survives_its_own_call() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = &mut SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        conn.batch_execute("PRAGMA journal_mode = WAL; CREATE TABLE borrowed_wal (x)")
            .unwrap();

        let calls = Arc::new(AtomicU32::new(0));
        let c2 = calls.clone();
        conn.on_wal(move |_conn, _db_name, _n_pages| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        conn.batch_execute("INSERT INTO borrowed_wal VALUES (1)")
            .unwrap();
        conn.batch_execute("INSERT INTO borrowed_wal VALUES (2)")
            .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    // Hooks registered inside a WAL callback are freed when it returns, also
    // when they fire from a nested borrowed connection inside that callback.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn hooks_registered_in_wal_callback_are_freed() {
        use crate::sqlite::CommitDecision;
        struct Token(Arc<AtomicU32>);
        impl Drop for Token {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = &mut SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        conn.batch_execute("PRAGMA journal_mode = WAL; CREATE TABLE borrowed_wal_hooks (x)")
            .unwrap();

        let dropped = Arc::new(AtomicU32::new(0));
        let inner_wal_calls = Arc::new(AtomicU32::new(0));
        let (d, w) = (dropped.clone(), inner_wal_calls.clone());
        conn.on_wal(move |conn, _db_name, _n_pages| {
            let conn = unsafe { conn.inner_mut() };
            let t = Token(d.clone());
            conn.on_commit(move || {
                let _ = &t;
                CommitDecision::Proceed
            });
            let (t, w) = (Token(d.clone()), w.clone());
            // Replaces the outer WAL hook while it runs; fires from a nested borrowed connection.
            conn.on_wal(move |_conn, _, _| {
                let _ = &t;
                w.fetch_add(1, Ordering::SeqCst);
            });
            conn.batch_execute("INSERT INTO borrowed_wal_hooks VALUES (0)")
                .unwrap();
        });

        conn.batch_execute("INSERT INTO borrowed_wal_hooks VALUES (1)")
            .unwrap();
        assert_eq!(inner_wal_calls.load(Ordering::SeqCst), 1);
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    }
}
