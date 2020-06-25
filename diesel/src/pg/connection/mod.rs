use crate::connection::{
    AnsiTransactionManager, Connection, MaybeCached, SimpleConnection, StatementCache,
};
use crate::deserialize::{FromSqlRow, Queryable, QueryableByName};
use crate::pg::metadata_lookup::PgMetadataCache;
use crate::pg::transaction::TransactionBuilder;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::query_dsl::load_dsl::CompatibleType;
use crate::result::ConnectionError::{self, CouldntSetupConfiguration};
use crate::result::Error::DeserializationError;
use crate::result::{ConnectionResult, QueryResult};
use crate::row::{NamedRow, Row};
use crate::sql_types::HasSqlType;
use bytes::BytesMut;
use postgresql::fallible_iterator::FallibleIterator;
use postgresql::types::Type;
use postgresql::Statement;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::error::Error;

/// This connection type is based on the pure rust postgresql implementation
/// inside the [postgres](https://docs.rs/postgres/0.17.0/postgres/index.html) crate.
///
/// See the documentation on [`postgres::Client`](https://docs.rs/postgres/0.19.0/postgres/struct.Client.html#method.connect)
/// for connection strings that are accepted by `PgConnection::establish`
/// or use the the `TryFrom` impl to create a new `PgConnection` from
/// an existing `postgres::Client`
#[allow(missing_debug_implementations)]
pub struct PgConnection {
    conn: RefCell<postgresql::Client>,
    pub(crate) transaction_manager: AnsiTransactionManager,
    statement_cache: StatementCache<Pg, Statement>,
    metadata_cache: PgMetadataCache,
}

impl PgConnection {
    /// Get a reference to the inner `postgres::Client`
    pub fn inner_connection(&self) -> &RefCell<postgresql::Client> {
        &self.conn
    }
}

impl TryFrom<postgresql::Client> for PgConnection {
    type Error = ConnectionError;

    fn try_from(client: postgresql::Client) -> Result<PgConnection, ConnectionError> {
        let conn = Self {
            conn: RefCell::new(client),
            transaction_manager: AnsiTransactionManager::new(),
            statement_cache: StatementCache::new(),
            metadata_cache: PgMetadataCache::new(),
        };

        conn.set_config_options()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }
}

impl SimpleConnection for PgConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.conn.borrow_mut().simple_query(query)?;
        Ok(())
    }
}

impl Connection for PgConnection {
    type Backend = super::Pg;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        use postgresql::tls::NoTls;

        let client = postgresql::Client::connect(database_url, NoTls)
            .map_err(|e| crate::result::ConnectionError::BadConnection(e.to_string()))?;
        let conn = Self {
            conn: RefCell::new(client),
            transaction_manager: AnsiTransactionManager::new(),
            statement_cache: StatementCache::new(),
            metadata_cache: PgMetadataCache::new(),
        };

        conn.set_config_options()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        Ok(self.conn.borrow_mut().execute(query, &[])? as usize)
    }

    #[doc(hidden)]
    fn load<T, U, ST>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        T::SqlType: CompatibleType<U, Self::Backend, SqlType = ST>,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        let (query, params) = self.prepare_query(&source.as_query())?;
        let params = params
            .into_iter()
            .map(|bytes| DieselToSqlWrapper { bytes })
            .collect::<Vec<_>>();
        let bind_params = params.iter().map(|a| a as &dyn postgresql::types::ToSql);

        let mut conn = self.conn.borrow_mut();
        let result_iter = conn.query_raw(&*query, bind_params)?;

        result_iter
            .map_err(crate::result::Error::from)
            .map(|row| Ok(PostgresRow { col_idx: 0, row }))
            .map(|mut row| U::Row::build_from_row(&mut row).map_err(DeserializationError))
            .map(|raw| Ok(U::build(raw)))
            .collect()
    }

    #[doc(hidden)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Self::Backend> + QueryId,
        U: QueryableByName<Self::Backend>,
    {
        let (query, params) = self.prepare_query(source)?;
        let params = params
            .into_iter()
            .map(|bytes| DieselToSqlWrapper { bytes })
            .collect::<Vec<_>>();
        let bind_params = params.iter().map(|a| a as &dyn postgresql::types::ToSql);

        let mut conn = self.conn.borrow_mut();
        let result_iter = conn.query_raw(&*query, bind_params)?;

        result_iter
            .map_err(crate::result::Error::from)
            .map(|row| Ok(NamedPostgresRow { row }))
            .map(|row| U::build(&row).map_err(DeserializationError))
            .collect()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let (query, params) = self.prepare_query(source)?;
        let params = params
            .into_iter()
            .map(|bytes| DieselToSqlWrapper { bytes })
            .collect::<Vec<_>>();
        let bind_params = params
            .iter()
            .map(|a| a as &(dyn postgresql::types::ToSql + Sync))
            .collect::<Vec<_>>();
        Ok(self
            .conn
            .borrow_mut()
            .execute(&*query, &bind_params)
            .map(|a| a as usize)?)
    }
    #[doc(hidden)]
    fn transaction_manager(&self) -> &dyn TransactionManager<Self> {
        &self.transaction_manager
    }
}

impl GetPgMetadataCache for PgConnection {
    fn get_metadata_cache(&self) -> &PgMetadataCache {
        &self.metadata_cache
    }
}

impl PgConnection {
    fn prepare_query<T: QueryFragment<Pg> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<(MaybeCached<Statement>, Vec<Option<Vec<u8>>>)> {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        source.collect_binds(&mut bind_collector, super::PgMetadataLookup::new(self))?;
        let metadata = &bind_collector.metadata;
        let binds = bind_collector.binds;
        let query = self
            .statement_cache
            .cached_statement(source, metadata, |sql| {
                if sql.is_empty() {
                    // No need to do anything with empty queries
                    return Err(crate::result::Error::DatabaseError(
                        crate::result::DatabaseErrorKind::__Unknown,
                        Box::new(String::from(
                            "Diesel does not support excecuting empty queries",
                        )),
                    ));
                }
                let metadata = metadata
                    .iter()
                    .map(|tpe_meta| {
                        Type::from_oid(tpe_meta.oid).unwrap_or_else(|| {
                            use postgresql::types::Kind;
                            Type::new(
                                tpe_meta.oid.to_string(),
                                tpe_meta.oid,
                                Kind::Simple,
                                "public".to_string(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                Ok(self.conn.borrow_mut().prepare_typed(sql, &metadata)?)
            });

        Ok((query?, binds))
    }

    fn set_config_options(&self) -> QueryResult<()> {
        self.execute("SET TIME ZONE 'UTC'")?;
        self.execute("SET CLIENT_ENCODING TO 'UTF8'")?;
        Ok(())
    }

    pub(crate) fn get_metadata_cache(&self) -> &PgMetadataCache {
        &self.metadata_cache
    }

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
}

#[derive(Debug)]
struct DieselToSqlWrapper {
    bytes: Option<Vec<u8>>,
}

impl postgresql::types::ToSql for DieselToSqlWrapper {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut BytesMut,
    ) -> Result<postgresql::types::IsNull, Box<dyn Error + 'static + Send + Sync>> {
        match self.bytes {
            Some(ref bytes) => {
                out.extend(bytes);
                Ok(postgresql::types::IsNull::No)
            }
            None => Ok(postgresql::types::IsNull::Yes),
        }
    }

    fn accepts(ty: &Type) -> bool {
        ty.oid() != 0
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<postgresql::types::IsNull, Box<dyn Error + 'static + Send + Sync>> {
        if !Self::accepts(ty) {
            todo!("write a proper error message")
        }
        self.to_sql(ty, out)
    }
}

struct PostgresRow {
    row: postgresql::row::Row,
    col_idx: usize,
}

impl Row<Pg> for PostgresRow {
    fn take(&mut self) -> Option<super::PgValue> {
        let current_index = self.col_idx;
        self.col_idx += 1;
        let DieselFromSqlWrapper(value) = self.row.get(current_index);
        value
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| {
            let DieselFromSqlWrapper(value) = self.row.get(self.col_idx + i);
            value.is_none()
        })
    }
}

struct DieselFromSqlWrapper<'a>(Option<super::PgValue<'a>>);

impl<'a> postgresql::types::FromSql<'a> for DieselFromSqlWrapper<'a> {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + 'static + Send + Sync>> {
        use std::num::NonZeroU32;

        Ok(DieselFromSqlWrapper(Some(super::PgValue::new(
            raw,
            NonZeroU32::new(ty.oid()).expect("That's not 0"),
        ))))
    }

    fn accepts(ty: &Type) -> bool {
        ty.oid() != 0
    }

    fn from_sql_null(_ty: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(DieselFromSqlWrapper(None))
    }
}

struct NamedPostgresRow {
    row: postgresql::row::Row,
}

impl NamedRow<Pg> for NamedPostgresRow {
    fn index_of(&self, column_name: &str) -> Option<usize> {
        self.row
            .columns()
            .iter()
            .position(|c| c.name() == column_name)
    }

    fn get_raw_value(&self, index: usize) -> Option<super::PgValue> {
        let DieselFromSqlWrapper(value) = self.row.get(index);
        value
    }
}

impl From<postgresql::error::Error> for crate::result::Error {
    fn from(e: postgresql::error::Error) -> crate::result::Error {
        use crate::result::DatabaseErrorKind::*;
        use postgresql::error::SqlState;

        if let Some(code) = e.code() {
            let kind = if *code == SqlState::UNIQUE_VIOLATION {
                UniqueViolation
            } else if *code == SqlState::FOREIGN_KEY_VIOLATION {
                ForeignKeyViolation
            } else if *code == SqlState::T_R_SERIALIZATION_FAILURE {
                SerializationFailure
            } else if *code == SqlState::READ_ONLY_SQL_TRANSACTION {
                ReadOnlyTransaction
            } else if *code == SqlState::NOT_NULL_VIOLATION {
                NotNullViolation
            } else if *code == SqlState::CHECK_VIOLATION {
                CheckViolation
            } else {
                __Unknown
            };

            crate::result::Error::DatabaseError(
                kind,
                e.into_source()
                    .and_then(|e| e.downcast::<postgresql::error::DbError>().ok())
                    .expect("It's a db error, because we've got a SQLState code above"),
            )
        } else {
            crate::result::Error::DatabaseError(UnableToSendCommand, Box::new(e.to_string()))
        }
    }
}

impl crate::result::DatabaseErrorInformation for postgresql::error::DbError {
    fn message(&self) -> &str {
        self.message()
    }

    fn details(&self) -> Option<&str> {
        self.detail()
    }

    fn hint(&self) -> Option<&str> {
        self.hint()
    }

    fn table_name(&self) -> Option<&str> {
        self.table()
    }

    fn column_name(&self) -> Option<&str> {
        self.column()
    }

    fn constraint_name(&self) -> Option<&str> {
        self.constraint()
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;

    use self::dotenv::dotenv;
    use std::env;

    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::result::Error::DatabaseError;
    use crate::sql_types::{Integer, VarChar};

    #[test]
    fn malformed_sql_query() {
        let connection = connection();
        let query =
            crate::sql_query("SELECT not_existent FROM also_not_there;").execute(&connection);

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

    sql_function!(fn lower(x: VarChar) -> VarChar);

    #[test]
    fn queries_with_identical_types_and_binds_but_different_sql_are_cached_separately() {
        let connection = connection();

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
