mod cursor;
pub mod raw;
#[doc(hidden)]
pub mod result;
mod row;
mod stmt;

use std::ffi::CString;
use std::os::raw as libc;

use self::cursor::*;
use self::raw::RawConnection;
use self::result::PgResult;
use self::stmt::Statement;
use crate::connection::*;
use crate::deserialize::{Queryable, QueryableByName};
use crate::pg::{Pg, PgMetadataLookup, TransactionBuilder};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::result::ConnectionError::CouldntSetupConfiguration;
use crate::result::*;
use crate::sql_types::HasSqlType;

/// The connection string expected by `PgConnection::establish`
/// should be a PostgreSQL connection string, as documented at
/// <https://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING>
#[allow(missing_debug_implementations)]
pub struct PgConnection {
    raw_connection: RawConnection,
    transaction_manager: AnsiTransactionManager,
    statement_cache: StatementCache<Pg, Statement>,
}

unsafe impl Send for PgConnection {}

impl SimpleConnection for PgConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        let query = CString::new(query)?;
        let inner_result = unsafe { self.raw_connection.exec(query.as_ptr()) };
        PgResult::new(inner_result?)?;
        Ok(())
    }
}

impl Connection for PgConnection {
    type Backend = Pg;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<PgConnection> {
        RawConnection::establish(database_url).and_then(|raw_conn| {
            let conn = PgConnection {
                raw_connection: raw_conn,
                transaction_manager: AnsiTransactionManager::new(),
                statement_cache: StatementCache::new(),
            };
            conn.set_config_options()
                .map_err(CouldntSetupConfiguration)?;
            Ok(conn)
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Pg> + QueryId,
        Pg: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Pg>,
    {
        let (query, params) = self.prepare_query(&source.as_query())?;
        query
            .execute(&self.raw_connection, &params)
            .and_then(|r| Cursor::new(r).collect())
    }

    #[doc(hidden)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Pg> + QueryId,
        U: QueryableByName<Pg>,
    {
        let (query, params) = self.prepare_query(source)?;
        query
            .execute(&self.raw_connection, &params)
            .and_then(|r| NamedCursor::new(r).collect())
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        let (query, params) = self.prepare_query(source)?;
        query
            .execute(&self.raw_connection, &params)
            .map(|r| r.rows_affected())
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }
}

impl PgConnection {
    /// Build a transaction, specifying additional details such as isolation level
    ///
    /// See [`TransactionBuilder`] for more examples.
    ///
    /// [`TransactionBuilder`]: ../pg/struct.TransactionBuilder.html
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
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
    ///     .serializable()
    ///     .deferrable()
    ///     .run(|| Ok(()))
    /// # }
    /// ```
    pub fn build_transaction(&self) -> TransactionBuilder {
        TransactionBuilder::new(self)
    }

    #[allow(clippy::type_complexity)]
    fn prepare_query<T: QueryFragment<Pg> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<(MaybeCached<Statement>, Vec<Option<Vec<u8>>>)> {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        source.collect_binds(&mut bind_collector, PgMetadataLookup::new(self))?;
        let binds = bind_collector.binds;
        let metadata = bind_collector.metadata;

        let cache_len = self.statement_cache.len();
        let query = self
            .statement_cache
            .cached_statement(source, &metadata, |sql| {
                let query_name = if source.is_safe_to_cache_prepared()? {
                    Some(format!("__diesel_stmt_{}", cache_len))
                } else {
                    None
                };
                Statement::prepare(
                    &self.raw_connection,
                    sql,
                    query_name.as_ref().map(|s| &**s),
                    &metadata,
                )
            });

        Ok((query?, binds))
    }

    fn execute_inner(&self, query: &str) -> QueryResult<PgResult> {
        let query = Statement::prepare(&self.raw_connection, query, None, &[])?;
        query.execute(&self.raw_connection, &Vec::new())
    }

    fn set_config_options(&self) -> QueryResult<()> {
        self.execute("SET TIME ZONE 'UTC'")?;
        self.execute("SET CLIENT_ENCODING TO 'UTF8'")?;
        self.raw_connection
            .set_notice_processor(noop_notice_processor);
        Ok(())
    }
}

extern "C" fn noop_notice_processor(_: *mut libc::c_void, _message: *const libc::c_char) {}

#[cfg(test)]
mod tests {
    extern crate dotenv;

    use self::dotenv::dotenv;
    use std::env;

    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::{Integer, VarChar};

    #[test]
    fn prepared_statements_are_cached() {
        let connection = connection();

        let query = crate::select(1.into_sql::<Integer>());

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_sql_but_different_types_are_cached_separately() {
        let connection = connection();

        let query = crate::select(1.into_sql::<Integer>());
        let query2 = crate::select("hi".into_sql::<VarChar>());

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(&connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_types_and_sql_but_different_bind_types_are_cached_separately() {
        let connection = connection();

        let query = crate::select(1.into_sql::<Integer>()).into_boxed::<Pg>();
        let query2 = crate::select("hi".into_sql::<VarChar>()).into_boxed::<Pg>();

        assert_eq!(0, connection.statement_cache.len());
        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(&connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_types_and_binds_but_different_sql_are_cached_separately() {
        let connection = connection();

        sql_function!(fn lower(x: VarChar) -> VarChar);
        let hi = "HI".into_sql::<VarChar>();
        let query = crate::select(hi).into_boxed::<Pg>();
        let query2 = crate::select(lower(hi)).into_boxed::<Pg>();

        assert_eq!(0, connection.statement_cache.len());
        assert_eq!(Ok("HI".to_string()), query.get_result(&connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(&connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_sql_literal_nodes_are_not_cached() {
        let connection = connection();
        let query = crate::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    fn connection() -> PgConnection {
        dotenv().ok();
        let database_url = env::var("PG_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        PgConnection::establish(&database_url).unwrap()
    }
}
