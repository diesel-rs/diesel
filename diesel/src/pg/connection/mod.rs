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
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::pg::metadata_lookup::{GetPgMetadataCache, PgMetadataCache};
use crate::pg::{Pg, TransactionBuilder};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::query_dsl::load_dsl::CompatibleType;
use crate::result::ConnectionError::CouldntSetupConfiguration;
use crate::result::Error::DeserializationError;
use crate::result::*;

/// The connection string expected by `PgConnection::establish`
/// should be a PostgreSQL connection string, as documented at
/// <https://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING>
#[allow(missing_debug_implementations)]
pub struct PgConnection {
    pub(crate) raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
    statement_cache: StatementCache<Pg, Statement>,
    metadata_cache: PgMetadataCache,
}

unsafe impl Send for PgConnection {}

impl SimpleConnection for PgConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
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
            let mut conn = PgConnection {
                raw_connection: raw_conn,
                transaction_state: AnsiTransactionManager::default(),
                statement_cache: StatementCache::new(),
                metadata_cache: PgMetadataCache::new(),
            };
            conn.set_config_options()
                .map_err(CouldntSetupConfiguration)?;
            Ok(conn)
        })
    }

    #[doc(hidden)]
    fn execute(&mut self, query: &str) -> QueryResult<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    #[doc(hidden)]
    fn load<T, U, ST>(&mut self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        T::SqlType: CompatibleType<U, Self::Backend, SqlType = ST>,
        U: FromSqlRow<ST, Self::Backend>,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        self.with_prepared_query(&source.as_query(), |stmt, params, conn| {
            let result = stmt.execute(conn, &params)?;
            let cursor = Cursor::new(&result);

            cursor
                .map(|row| U::build_from_row(&row).map_err(DeserializationError))
                .collect::<QueryResult<Vec<_>>>()
        })
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        self.with_prepared_query(source, |query, params, conn| {
            query.execute(conn, &params).map(|r| r.rows_affected())
        })
    }

    fn transaction_state(&mut self) -> &mut AnsiTransactionManager
    where
        Self: Sized,
    {
        &mut self.transaction_state
    }
}

impl GetPgMetadataCache for PgConnection {
    fn get_metadata_cache(&mut self) -> &mut PgMetadataCache {
        &mut self.metadata_cache
    }
}

impl PgConnection {
    /// Build a transaction, specifying additional details such as isolation level
    ///
    /// See [`TransactionBuilder`] for more examples.
    ///
    /// [`TransactionBuilder`]: crate::pg::TransactionBuilder
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut connection_no_transaction();
    /// conn.build_transaction()
    ///     .read_only()
    ///     .serializable()
    ///     .deferrable()
    ///     .run(|conn| Ok(()))
    /// # }
    /// ```
    pub fn build_transaction(&mut self) -> TransactionBuilder<Self> {
        TransactionBuilder::new(self)
    }

    fn with_prepared_query<T: QueryFragment<Pg> + QueryId, R>(
        &mut self,
        source: &T,
        f: impl FnOnce(
            MaybeCached<Statement>,
            Vec<Option<Vec<u8>>>,
            &mut RawConnection,
        ) -> QueryResult<R>,
    ) -> QueryResult<R> {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        source.collect_binds(&mut bind_collector, self)?;
        let binds = bind_collector.binds;
        let metadata = bind_collector.metadata;

        let cache_len = self.statement_cache.len();
        let cache = &mut self.statement_cache;
        let raw_conn = &mut self.raw_connection;
        let query = cache.cached_statement(source, &metadata, |sql| {
            let query_name = if source.is_safe_to_cache_prepared()? {
                Some(format!("__diesel_stmt_{}", cache_len))
            } else {
                None
            };
            Statement::prepare(raw_conn, sql, query_name.as_deref(), &metadata)
        });

        f(query?, binds, raw_conn)
    }

    fn execute_inner(&mut self, query: &str) -> QueryResult<PgResult> {
        let query = Statement::prepare(&mut self.raw_connection, query, None, &[])?;
        query.execute(&mut self.raw_connection, &Vec::new())
    }

    fn set_config_options(&mut self) -> QueryResult<()> {
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

    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::result::Error::DatabaseError;
    use crate::sql_types::{Integer, VarChar};

    #[test]
    fn malformed_sql_query() {
        let connection = &mut connection();
        let query =
            crate::sql_query("SELECT not_existent FROM also_not_there;").execute(connection);

        if let Err(err) = query {
            if let DatabaseError(_, string) = err {
                assert_eq!(Some(26), string.statement_position());
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        }
    }

    #[test]
    fn prepared_statements_are_cached() {
        let connection = &mut connection();

        let query = crate::select(1.into_sql::<Integer>());

        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_sql_but_different_types_are_cached_separately() {
        let connection = &mut connection();

        let query = crate::select(1.into_sql::<Integer>());
        let query2 = crate::select("hi".into_sql::<VarChar>());

        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_types_and_sql_but_different_bind_types_are_cached_separately() {
        let connection = &mut connection();

        let query = crate::select(1.into_sql::<Integer>()).into_boxed::<Pg>();
        let query2 = crate::select("hi".into_sql::<VarChar>()).into_boxed::<Pg>();

        assert_eq!(0, connection.statement_cache.len());
        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    sql_function!(fn lower(x: VarChar) -> VarChar);

    #[test]
    fn queries_with_identical_types_and_binds_but_different_sql_are_cached_separately() {
        let connection = &mut connection();

        let hi = "HI".into_sql::<VarChar>();
        let query = crate::select(hi).into_boxed::<Pg>();
        let query2 = crate::select(lower(hi)).into_boxed::<Pg>();

        assert_eq!(0, connection.statement_cache.len());
        assert_eq!(Ok("HI".to_string()), query.get_result(connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_sql_literal_nodes_are_not_cached() {
        let connection = &mut connection();
        let query = crate::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    table! {
        users {
            id -> Integer,
            name -> Text,
        }
    }

    #[test]
    fn inserts_from_select_are_cached() {
        let connection = &mut connection();
        connection.begin_test_transaction().unwrap();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(connection)
        .unwrap();

        let query = users::table.filter(users::id.eq(42));
        let insert = query
            .insert_into(users::table)
            .into_columns((users::id, users::name));
        assert_eq!(true, insert.execute(connection).is_ok());
        assert_eq!(1, connection.statement_cache.len());

        let query = users::table.filter(users::id.eq(42)).into_boxed();
        let insert = query
            .insert_into(users::table)
            .into_columns((users::id, users::name));
        assert_eq!(true, dbg!(insert.execute(connection)).is_ok());
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn single_inserts_are_cached() {
        let connection = &mut connection();
        connection.begin_test_transaction().unwrap();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(connection)
        .unwrap();

        let insert =
            crate::insert_into(users::table).values((users::id.eq(42), users::name.eq("Foo")));

        assert_eq!(true, insert.execute(connection).is_ok());
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn dynamic_batch_inserts_are_not_cached() {
        let connection = &mut connection();
        connection.begin_test_transaction().unwrap();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(connection)
        .unwrap();

        let insert = crate::insert_into(users::table)
            .values(vec![(users::id.eq(42), users::name.eq("Foo"))]);

        assert_eq!(true, insert.execute(connection).is_ok());
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn static_batch_inserts_are_cached() {
        let connection = &mut connection();
        connection.begin_test_transaction().unwrap();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(connection)
        .unwrap();

        let insert =
            crate::insert_into(users::table).values([(users::id.eq(42), users::name.eq("Foo"))]);

        assert_eq!(true, insert.execute(connection).is_ok());
        assert_eq!(1, connection.statement_cache.len());
    }

    fn connection() -> PgConnection {
        crate::test_helpers::pg_connection_no_transaction()
    }
}
