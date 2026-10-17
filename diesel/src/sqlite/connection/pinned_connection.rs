// this is safe as we don't replace the connection
// but only call functions
//
// All of this is safe as long as we don't replace the sqlite3
// pointer
#![allow(unsafe_code)]

use super::{SqliteConnection, WalCheckpointMode, WalCheckpointOutcome};
use crate::connection::*;
use crate::expression::QueryMetadata;
use crate::prelude::{ConnectionResult, QueryResult};
use crate::query_builder::{Query, QueryFragment, QueryId};
use crate::sqlite::Sqlite;
use core::pin::Pin;

impl SimpleConnection for Pin<&mut SqliteConnection> {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::batch_execute(mut_ref, query)
    }
}

impl Connection for Pin<&mut SqliteConnection> {
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
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::execute_returning_count(mut_ref, source)
    }

    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::transaction_state(mut_ref)
    }

    fn instrumentation(&mut self) -> &mut dyn Instrumentation {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::instrumentation(mut_ref)
    }

    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::set_instrumentation(mut_ref, instrumentation);
    }

    fn set_prepared_statement_cache_size(&mut self, size: CacheSize) {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::set_prepared_statement_cache_size(mut_ref, size)
    }
}

impl LoadConnection for Pin<&mut SqliteConnection> {
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
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::load(mut_ref, source)
    }
}

impl ConnectionSealed for Pin<&mut SqliteConnection> {}

/// A helper trait to expose safe methods from `SqliteConnection` for `Pin<&mut SqliteConnection>`
pub trait PinnedSqliteConnectionMethods: Sealed {
    /// See `SqliteConnection::deserialize_readonly_database_from_buffer`
    fn deserialize_readonly_database_from_buffer(&mut self, data: &[u8]) -> QueryResult<()>;

    /// See `SqliteConnection::register_collation`
    fn register_collation<F>(&mut self, collation_name: &str, collation: F) -> QueryResult<()>
    where
        F: Fn(&str, &str) -> core::cmp::Ordering + Send + 'static + core::panic::UnwindSafe;

    /// See `SqliteConnection::wal_checkpoint`
    fn wal_checkpoint(
        &mut self,
        schema: Option<&str>,
        mode: WalCheckpointMode,
    ) -> QueryResult<WalCheckpointOutcome>;
}

impl Sealed for Pin<&mut SqliteConnection> {}

impl PinnedSqliteConnectionMethods for Pin<&mut SqliteConnection> {
    fn deserialize_readonly_database_from_buffer(&mut self, data: &[u8]) -> QueryResult<()> {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::deserialize_readonly_database_from_buffer(mut_ref, data)
    }

    fn register_collation<F>(&mut self, collation_name: &str, collation: F) -> QueryResult<()>
    where
        F: Fn(&str, &str) -> core::cmp::Ordering + Send + 'static + core::panic::UnwindSafe,
    {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::register_collation(mut_ref, collation_name, collation)
    }

    fn wal_checkpoint(
        &mut self,
        schema: Option<&str>,
        mode: WalCheckpointMode,
    ) -> QueryResult<WalCheckpointOutcome> {
        let mut_ref = unsafe {
            // SAFETY: this relies on the fact
            // that non of the calls done with the references
            // below doesn't replace the connection, but
            // only modify inner fields
            self.as_mut().get_unchecked_mut()
        };
        SqliteConnection::wal_checkpoint(mut_ref, schema, mode)
    }
}

pub trait Sealed {}
