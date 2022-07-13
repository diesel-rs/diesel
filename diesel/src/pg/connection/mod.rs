pub(crate) mod cursor;
mod raw;
mod result;
mod row;
mod stmt;

use std::ffi::CString;
use std::os::raw as libc;

use self::cursor::*;
use self::raw::{PgTransactionStatus, RawConnection};
use self::result::PgResult;
use self::stmt::Statement;
use crate::connection::statement_cache::{MaybeCached, StatementCache};
use crate::connection::*;
use crate::expression::QueryMetadata;
use crate::pg::metadata_lookup::{GetPgMetadataCache, PgMetadataCache};
use crate::pg::{Pg, TransactionBuilder};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::result::ConnectionError::CouldntSetupConfiguration;
use crate::result::*;
use crate::RunQueryDsl;

/// The connection string expected by `PgConnection::establish`
/// should be a PostgreSQL connection string, as documented at
/// <https://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING>
///
/// # Supported loading model implementations
///
/// * [`DefaultLoadingMode`]
/// * [`PgRowByRowLoadingMode`]
///
/// If you are unsure which loading mode is the correct one for your application,
/// you likely want to use the `DefaultLoadingMode` as that one offers
/// generally better performance.
///
/// Due to the fact that `PgConnection` supports multiple loading modes
/// it is **required** to always specify the used loading mode
/// when calling [`RunQueryDsl::load_iter`] or [`LoadConnection::load`].
///
/// ## `DefaultLoadingMode`
///
/// By using this mode `PgConnection` defaults to loading all response values at **once**
/// and only performs deserialization afterward for the `DefaultLoadingMode`.
/// Generally this mode will be more performant as it.
///
/// This loading mode allows users to perform hold more than one iterator at once using
/// the same connection:
/// ```rust
/// # include!("../../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users;
/// #     let connection = &mut establish_connection();
/// use diesel::connection::DefaultLoadingMode;
///
/// let iter1 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
/// let iter2 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
///
/// for r in iter1 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
///
/// for r in iter2 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
/// #   Ok(())
/// # }
/// ```
///
/// ## `PgRowByRowLoadingMode`
///
/// By using this mode `PgConnection` defaults to loading each row of the result set
/// sepreatly. This might be desired for huge result sets.
///
/// This loading mode **prevents** creating more than one iterator at once using
/// the same connection. The following code is **not** allowed:
///
/// ```compile_fail
/// # include!("../../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users;
/// #     let connection = &mut establish_connection();
/// use diesel::pg::PgRowByRowLoadingMode;
///
/// let iter1 = users::table.load_iter::<(i32, String), PgRowByRowLoadingMode>(connection)?;
/// // creating a second iterator generates an compiler error
/// let iter2 = users::table.load_iter::<(i32, String), PgRowByRowLoadingMode>(connection)?;
///
/// for r in iter1 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
///
/// for r in iter2 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
/// #   Ok(())
/// # }
/// ```
#[allow(missing_debug_implementations)]
#[cfg(feature = "postgres")]
pub struct PgConnection {
    raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
    statement_cache: StatementCache<Pg, Statement>,
    metadata_cache: PgMetadataCache,
}

unsafe impl Send for PgConnection {}

impl SimpleConnection for PgConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        let query = CString::new(query)?;
        let inner_result = unsafe { self.raw_connection.exec(query.as_ptr()) };
        update_transaction_manager_status(
            inner_result.and_then(|raw_result| PgResult::new(raw_result, &self.raw_connection)),
            self,
        )?;
        Ok(())
    }
}

/// A [`PgConnection`] specific loading mode to load rows one by one
///
/// See the documentation of [`PgConnection`] for details
#[derive(Debug, Copy, Clone)]
pub struct PgRowByRowLoadingMode;

impl<'conn, 'query> ConnectionGatWorkaround<'conn, 'query, Pg, DefaultLoadingMode>
    for PgConnection
{
    type Cursor = Cursor;
    type Row = self::row::PgRow;
}

impl<'conn, 'query> ConnectionGatWorkaround<'conn, 'query, Pg, PgRowByRowLoadingMode>
    for PgConnection
{
    type Cursor = RowByRowCursor<'conn>;
    type Row = self::row::PgRow;
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
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        update_transaction_manager_status(
            self.with_prepared_query(source, |query, params, conn| {
                let res = query
                    .execute(conn, &params, false)
                    .map(|r| r.rows_affected());
                // TODO: understand why this is required
                let next_res = conn.get_next_result();
                debug_assert!(matches!(next_res, Ok(None)));
                res
            }),
            self,
        )
    }

    fn transaction_state(&mut self) -> &mut AnsiTransactionManager
    where
        Self: Sized,
    {
        &mut self.transaction_state
    }
}

impl<B> LoadConnection<B> for PgConnection
where
    Self: self::private::PgLoadingMode<B>,
{
    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<LoadRowIter<'conn, 'query, Self, Self::Backend, B>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        self.with_prepared_query(&source, |stmt, params, conn| {
            use self::private::PgLoadingMode;
            let result = stmt.execute(conn, &params, Self::USE_ROW_BY_ROW_MODE)?;
            let cursor = Self::get_cursor(conn, result);

            cursor
        })
    }
}

impl GetPgMetadataCache for PgConnection {
    fn get_metadata_cache(&mut self) -> &mut PgMetadataCache {
        &mut self.metadata_cache
    }
}

#[inline(always)]
fn update_transaction_manager_status<T>(
    query_result: QueryResult<T>,
    conn: &mut PgConnection,
) -> QueryResult<T> {
    if let Err(Error::DatabaseError { .. }) = query_result {
        /// avoid monomorphizing for every result type - this part will not be inlined
        fn non_generic_inner(conn: &mut PgConnection) {
            if AnsiTransactionManager::transaction_manager_status_mut(conn)
                .is_not_broken_and_in_transaction()
            {
                // libpq keeps track of the transaction status internally, and that is accessible
                // via `transaction_status`. We can use that to update the AnsiTransactionManager
                // status
                match conn.raw_connection.transaction_status() {
                    PgTransactionStatus::InError => {
                        AnsiTransactionManager::transaction_manager_status_mut(conn)
                            .set_top_level_transaction_requires_rollback()
                    }
                    PgTransactionStatus::Unknown => {
                        AnsiTransactionManager::transaction_manager_status_mut(conn).set_in_error()
                    }
                    PgTransactionStatus::Idle => {
                        // This may repair the transaction manager
                        *AnsiTransactionManager::transaction_manager_status_mut(conn) =
                            TransactionManagerStatus::Valid(Default::default())
                    }
                    PgTransactionStatus::InTransaction => {
                        let transaction_status =
                            AnsiTransactionManager::transaction_manager_status_mut(conn);
                        if !matches!(transaction_status, TransactionManagerStatus::Valid(valid_tm) if valid_tm.transaction_depth().is_some())
                        {
                            transaction_status.set_in_error()
                        }
                    }
                    PgTransactionStatus::Active => {}
                }
            }
        }
        non_generic_inner(conn)
    }
    query_result
}

#[cfg(feature = "r2d2")]
impl crate::r2d2::R2D2Connection for PgConnection {
    fn ping(&mut self) -> QueryResult<()> {
        crate::r2d2::CheckConnectionQuery.execute(self).map(|_| ())
    }

    fn is_broken(&mut self) -> bool {
        match self.transaction_state.status.transaction_depth() {
            // all transactions are closed
            // so we don't consider this connection broken
            Ok(None) => false,
            // The transaction manager is in an error state
            // or contains an open transaction
            // Therefore we consider this connection broken
            Err(_) | Ok(Some(_)) => true,
        }
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
    pub fn build_transaction(&mut self) -> TransactionBuilder<'_, Self> {
        TransactionBuilder::new(self)
    }

    fn with_prepared_query<'conn, T: QueryFragment<Pg> + QueryId, R>(
        &'conn mut self,
        source: &'_ T,
        f: impl FnOnce(
            MaybeCached<'_, Statement>,
            Vec<Option<Vec<u8>>>,
            &'conn mut RawConnection,
        ) -> QueryResult<R>,
    ) -> QueryResult<R> {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        source.collect_binds(&mut bind_collector, self, &Pg)?;
        let binds = bind_collector.binds;
        let metadata = bind_collector.metadata;

        let cache_len = self.statement_cache.len();
        let cache = &mut self.statement_cache;
        let raw_conn = &mut self.raw_connection;
        let query = cache.cached_statement(source, &Pg, &metadata, |sql, _| {
            let query_name = if source.is_safe_to_cache_prepared(&Pg)? {
                Some(format!("__diesel_stmt_{}", cache_len))
            } else {
                None
            };
            Statement::prepare(raw_conn, sql, query_name.as_deref(), &metadata)
        });

        f(query?, binds, &mut *raw_conn)
    }

    fn set_config_options(&mut self) -> QueryResult<()> {
        crate::sql_query("SET TIME ZONE 'UTC'").execute(self)?;
        crate::sql_query("SET CLIENT_ENCODING TO 'UTF8'").execute(self)?;
        self.raw_connection
            .set_notice_processor(noop_notice_processor);
        Ok(())
    }
}

extern "C" fn noop_notice_processor(_: *mut libc::c_void, _message: *const libc::c_char) {}

mod private {
    use super::*;

    pub trait PgLoadingMode<B>
    where
        for<'conn, 'query> Self: ConnectionGatWorkaround<'conn, 'query, Pg, B>,
    {
        const USE_ROW_BY_ROW_MODE: bool;

        fn get_cursor<'conn, 'query>(
            raw_connection: &'conn mut RawConnection,
            result: PgResult,
        ) -> QueryResult<<Self as ConnectionGatWorkaround<'conn, 'query, Pg, B>>::Cursor>;
    }

    impl PgLoadingMode<DefaultLoadingMode> for PgConnection {
        const USE_ROW_BY_ROW_MODE: bool = false;

        fn get_cursor<'conn, 'query>(
            raw_connection: &'conn mut RawConnection,
            result: PgResult,
        ) -> QueryResult<
            <Self as ConnectionGatWorkaround<'conn, 'query, Pg, DefaultLoadingMode>>::Cursor,
        > {
            Cursor::new(result, raw_connection)
        }
    }

    impl PgLoadingMode<PgRowByRowLoadingMode> for PgConnection {
        const USE_ROW_BY_ROW_MODE: bool = true;

        fn get_cursor<'conn, 'query>(
            raw_connection: &'conn mut RawConnection,
            result: PgResult,
        ) -> QueryResult<
            <Self as ConnectionGatWorkaround<'conn, 'query, Pg, PgRowByRowLoadingMode>>::Cursor,
        > {
            Ok(RowByRowCursor::new(result, raw_connection))
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::result::Error::DatabaseError;
    use crate::sql_types::{Integer, VarChar};
    use std::num::NonZeroU32;

    #[test]
    fn malformed_sql_query() {
        let connection = &mut connection();
        let query =
            crate::sql_query("SELECT not_existent FROM also_not_there;").execute(connection);

        if let Err(DatabaseError(_, string)) = query {
            assert_eq!(Some(26), string.statement_position());
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
        assert!(insert.execute(connection).is_ok());
        assert_eq!(1, connection.statement_cache.len());

        let query = users::table.filter(users::id.eq(42)).into_boxed();
        let insert = query
            .insert_into(users::table)
            .into_columns((users::id, users::name));
        assert!(insert.execute(connection).is_ok());
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

        assert!(insert.execute(connection).is_ok());
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

        assert!(insert.execute(connection).is_ok());
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

        assert!(insert.execute(connection).is_ok());
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_vec_are_cached() {
        let connection = &mut connection();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = crate::select(one_as_expr.eq_any(vec![1, 2, 3]));

        assert_eq!(Ok(true), query.get_result(connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    fn connection() -> PgConnection {
        crate::test_helpers::pg_connection_no_transaction()
    }

    #[test]
    fn transaction_manager_returns_an_error_when_attempting_to_commit_outside_of_a_transaction() {
        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::result::Error;
        use crate::PgConnection;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = AnsiTransactionManager::commit_transaction(conn);
        assert!(matches!(result, Err(Error::NotInTransaction)))
    }

    #[test]
    fn transaction_manager_returns_an_error_when_attempting_to_rollback_outside_of_a_transaction() {
        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::result::Error;
        use crate::PgConnection;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = AnsiTransactionManager::rollback_transaction(conn);
        assert!(matches!(result, Err(Error::NotInTransaction)))
    }

    #[test]
    fn postgres_transaction_is_rolled_back_upon_syntax_error() {
        use std::num::NonZeroU32;

        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::*;
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
                ).transaction_depth().expect("Transaction depth")
            );
            // In Postgres, a syntax error breaks the transaction block
            let query_result = sql_query("SELECT_SYNTAX_ERROR 1").execute(conn);
            assert!(query_result.is_err());
            assert_eq!(
                PgTransactionStatus::InError,
                conn.raw_connection.transaction_status()
            );
            query_result
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
    }

    #[test]
    fn nested_postgres_transaction_is_rolled_back_upon_syntax_error() {
        use std::num::NonZeroU32;

        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::*;
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let result = conn.build_transaction().run(|conn| {
                assert_eq!(
                    NonZeroU32::new(2),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
            ).transaction_depth().expect("Transaction depth")
                );
                sql_query("SELECT_SYNTAX_ERROR 1").execute(conn)
            });
            assert!(result.is_err());
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let query_result = sql_query("SELECT 1").execute(conn);
            assert!(query_result.is_ok());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            query_result
        });
        assert!(result.is_ok());
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    // This function uses collect with an side effect (spawning threads)
    // so this is a false positive from clippy
    #[allow(clippy::needless_collect)]
    fn postgres_transaction_depth_is_tracked_properly_on_serialization_failure() {
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::DatabaseErrorKind::SerializationFailure;
        use crate::result::Error::DatabaseError;
        use crate::*;
        use std::sync::{Arc, Barrier};
        use std::thread;

        table! {
            #[sql_name = "pg_transaction_depth_is_tracked_properly_on_commit_failure"]
            serialization_example {
                id -> Serial,
                class -> Integer,
            }
        }

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();

        sql_query(
            "DROP TABLE IF EXISTS pg_transaction_depth_is_tracked_properly_on_commit_failure;",
        )
        .execute(conn)
        .unwrap();
        sql_query(
            r#"
            CREATE TABLE pg_transaction_depth_is_tracked_properly_on_commit_failure (
                id SERIAL PRIMARY KEY,
                class INTEGER NOT NULL
            )
        "#,
        )
        .execute(conn)
        .unwrap();

        insert_into(serialization_example::table)
            .values(&vec![
                serialization_example::class.eq(1),
                serialization_example::class.eq(2),
            ])
            .execute(conn)
            .unwrap();

        let before_barrier = Arc::new(Barrier::new(2));
        let after_barrier = Arc::new(Barrier::new(2));
        let threads = (1..3)
            .map(|i| {
                let before_barrier = before_barrier.clone();
                let after_barrier = after_barrier.clone();
                thread::spawn(move || {
                    use crate::connection::AnsiTransactionManager;
                    use crate::connection::TransactionManager;
                    let conn = &mut crate::test_helpers::pg_connection_no_transaction();
                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                    let result = conn.build_transaction().serializable().run(|conn| {
                        assert_eq!(NonZeroU32::new(1), <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                        let _ = serialization_example::table
                            .filter(serialization_example::class.eq(i))
                            .count()
                            .execute(conn)?;

                        let other_i = if i == 1 { 2 } else { 1 };
                        let q = insert_into(serialization_example::table)
                            .values(serialization_example::class.eq(other_i));
                        before_barrier.wait();

                        let r = q.execute(conn);
                        after_barrier.wait();
                        r
                    });
                    assert_eq!(
                        PgTransactionStatus::Idle,
                        conn.raw_connection.transaction_status()
                    );

                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                    result
                })
            })
            .collect::<Vec<_>>();

        let mut results = threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();

        results.sort_by_key(|r| r.is_err());

        assert!(matches!(results[0], Ok(_)), "Got {:?} instead", results);
        assert!(
            matches!(&results[1], Err(DatabaseError(SerializationFailure, _))),
            "Got {:?} instead",
            results
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
    }

    #[test]
    // This function uses collect with an side effect (spawning threads)
    // so this is a false positive from clippy
    #[allow(clippy::needless_collect)]
    fn postgres_transaction_depth_is_tracked_properly_on_nested_serialization_failure() {
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::DatabaseErrorKind::SerializationFailure;
        use crate::result::Error::DatabaseError;
        use crate::*;
        use std::sync::{Arc, Barrier};
        use std::thread;

        table! {
            #[sql_name = "pg_nested_transaction_depth_is_tracked_properly_on_commit_failure"]
            serialization_example {
                id -> Serial,
                class -> Integer,
            }
        }

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();

        sql_query(
            "DROP TABLE IF EXISTS pg_nested_transaction_depth_is_tracked_properly_on_commit_failure;",
        )
        .execute(conn)
        .unwrap();
        sql_query(
            r#"
            CREATE TABLE pg_nested_transaction_depth_is_tracked_properly_on_commit_failure (
                id SERIAL PRIMARY KEY,
                class INTEGER NOT NULL
            )
        "#,
        )
        .execute(conn)
        .unwrap();

        insert_into(serialization_example::table)
            .values(&vec![
                serialization_example::class.eq(1),
                serialization_example::class.eq(2),
            ])
            .execute(conn)
            .unwrap();

        let before_barrier = Arc::new(Barrier::new(2));
        let after_barrier = Arc::new(Barrier::new(2));
        let threads = (1..3)
            .map(|i| {
                let before_barrier = before_barrier.clone();
                let after_barrier = after_barrier.clone();
                thread::spawn(move || {
                    use crate::connection::AnsiTransactionManager;
                    use crate::connection::TransactionManager;
                    let conn = &mut crate::test_helpers::pg_connection_no_transaction();
                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                    let result = conn.build_transaction().serializable().run(|conn| {
                        assert_eq!(NonZeroU32::new(1), <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                        let r = conn.transaction(|conn| {
                            assert_eq!(NonZeroU32::new(2), <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                            let _ = serialization_example::table
                                .filter(serialization_example::class.eq(i))
                                .count()
                                .execute(conn)?;

                            let other_i = if i == 1 { 2 } else { 1 };
                            let q = insert_into(serialization_example::table)
                                .values(serialization_example::class.eq(other_i));
                            before_barrier.wait();

                            let r = q.execute(conn);
                            after_barrier.wait();
                            r
                        });
                        assert_eq!(NonZeroU32::new(1), <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                        assert_eq!(
                            PgTransactionStatus::InTransaction,
                            conn.raw_connection.transaction_status()
                        );
                        r
                    });
                    assert_eq!(
                        PgTransactionStatus::Idle,
                        conn.raw_connection.transaction_status()
                    );

                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                    result
                })
            })
            .collect::<Vec<_>>();

        let mut results = threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();

        results.sort_by_key(|r| r.is_err());

        assert!(matches!(results[0], Ok(_)), "Got {:?} instead", results);
        assert!(
            matches!(&results[1], Err(DatabaseError(SerializationFailure, _))),
            "Got {:?} instead",
            results
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
    }

    #[test]
    fn postgres_transaction_is_rolled_back_upon_deferred_constraint_failure() {
        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_constraint_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_constraint_commit(id INT UNIQUE INITIALLY DEFERRED)")
                .execute(conn)?;
            sql_query("INSERT INTO deferred_constraint_commit VALUES(1)").execute(conn)?;
            let result =
                sql_query("INSERT INTO deferred_constraint_commit VALUES(1)").execute(conn);
            assert!(result.is_ok());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            Ok(())
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_err());
    }

    #[test]
    fn postgres_transaction_is_rolled_back_upon_deferred_trigger_failure() {
        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_trigger_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_trigger_commit(id INT UNIQUE INITIALLY DEFERRED)")
                .execute(conn)?;
            sql_query(
                r#"
                    CREATE OR REPLACE FUNCTION transaction_depth_blow_up()
                        RETURNS trigger
                        LANGUAGE plpgsql
                        AS $$
                    DECLARE
                    BEGIN
                        IF NEW.value = 42 THEN
                            RAISE EXCEPTION 'Transaction kaboom';
                        END IF;
                    RETURN NEW;

                    END;$$;
                "#,
            )
            .execute(conn)?;

            sql_query(
                r#"
                    CREATE CONSTRAINT TRIGGER transaction_depth_trigger
                        AFTER INSERT ON "deferred_trigger_commit"
                        DEFERRABLE INITIALLY DEFERRED
                        FOR EACH ROW
                        EXECUTE PROCEDURE transaction_depth_blow_up()
            "#,
            )
            .execute(conn)?;
            let result = sql_query("INSERT INTO deferred_trigger_commit VALUES(42)").execute(conn);
            assert!(result.is_ok());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            Ok(())
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_err());
    }

    #[test]
    fn nested_postgres_transaction_is_rolled_back_upon_deferred_trigger_failure() {
        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_trigger_nested_commit").execute(conn)?;
            sql_query(
                "CREATE TABLE deferred_trigger_nested_commit(id INT UNIQUE INITIALLY DEFERRED)",
            )
            .execute(conn)?;
            sql_query(
                r#"
                    CREATE OR REPLACE FUNCTION transaction_depth_blow_up()
                        RETURNS trigger
                        LANGUAGE plpgsql
                        AS $$
                    DECLARE
                    BEGIN
                        IF NEW.value = 42 THEN
                            RAISE EXCEPTION 'Transaction kaboom';
                        END IF;
                    RETURN NEW;

                    END;$$;
                "#,
            )
            .execute(conn)?;

            sql_query(
                r#"
                    CREATE CONSTRAINT TRIGGER transaction_depth_trigger
                        AFTER INSERT ON "deferred_trigger_nested_commit"
                        DEFERRABLE INITIALLY DEFERRED
                        FOR EACH ROW
                        EXECUTE PROCEDURE transaction_depth_blow_up()
            "#,
            )
            .execute(conn)?;
            let inner_result: Result<_, Error> = conn.build_transaction().run(|conn| {
                let result = sql_query("INSERT INTO deferred_trigger_nested_commit VALUES(42)")
                    .execute(conn);
                assert!(result.is_ok());
                Ok(())
            });
            assert!(inner_result.is_err());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            Ok(())
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_ok(), "Expected success, got {:?}", result);
    }

    #[test]
    fn nested_postgres_transaction_is_rolled_back_upon_deferred_constraint_failure() {
        use crate::connection::AnsiTransactionManager;
        use crate::connection::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_constraint_nested_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_constraint_nested_commit(id INT UNIQUE INITIALLY DEFERRED)").execute(conn)?;
            let inner_result: Result<_, Error> = conn.build_transaction().run(|conn| {
                assert_eq!(
                    NonZeroU32::new(2),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
                    ).transaction_depth().expect("Transaction depth")
                );
                sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn)?;
                let result = sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn);
                assert!(result.is_ok());
                Ok(())
            });
            assert!(inner_result.is_err());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn)
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_ok());
    }
}
