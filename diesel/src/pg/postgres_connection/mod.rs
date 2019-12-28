extern crate bytes;
extern crate postgresql;

use super::{Pg, PgMetadataLookup, PgValue};
use crate::connection::{
    AnsiTransactionManager, Connection, MaybeCached, SimpleConnection, StatementCache,
};
use crate::deserialize::{FromSqlRow, Queryable, QueryableByName};
use crate::pg::metadata_lookup::PgMetadataCache;
use crate::pg::transaction::TransactionBuilder;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::result::ConnectionError::CouldntSetupConfiguration;
use crate::result::Error::DeserializationError;
use crate::result::{ConnectionResult, QueryResult};
use crate::row::{NamedRow, Row};
use crate::sql_types::HasSqlType;
use bytes::BytesMut;
use postgresql::fallible_iterator::FallibleIterator;
use postgresql::types::Type;
use postgresql::Statement;
use std::cell::RefCell;
use std::error::Error;

#[allow(missing_docs, missing_debug_implementations)]
pub struct PostgresConnection {
    conn: RefCell<postgresql::Client>,
    transaction_manager: AnsiTransactionManager,
    statement_cache: StatementCache<Pg, Statement>,
    metadata_cache: PgMetadataCache,
}

impl SimpleConnection for PostgresConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.conn.borrow_mut().simple_query(query)?;
        Ok(())
    }
}

impl Connection for PostgresConnection {
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

    fn execute(&self, query: &str) -> QueryResult<usize> {
        Ok(self.conn.borrow_mut().execute(query, &[])? as usize)
    }

    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
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
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }
}

impl PostgresConnection {
    fn prepare_query<T: QueryFragment<Pg> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<(MaybeCached<Statement>, Vec<Option<Vec<u8>>>)> {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        source.collect_binds(&mut bind_collector, PgMetadataLookup::new(self))?;
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

    pub fn build_transaction(&self) -> TransactionBuilder<Self> {
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
        // improve that
        ty.oid() != 0
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<postgresql::types::IsNull, Box<dyn Error + 'static + Send + Sync>> {
        if !Self::accepts(ty) {
            todo!("write a proper error message")
        } else {
            self.to_sql(ty, out)
        }
    }
}

struct PostgresRow {
    row: postgresql::row::Row,
    col_idx: usize,
}

impl Row<Pg> for PostgresRow {
    fn take<'a>(&'a mut self) -> Option<PgValue<'a>> {
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

struct DieselFromSqlWrapper<'a>(Option<PgValue<'a>>);

impl<'a> postgresql::types::FromSql<'a> for DieselFromSqlWrapper<'a> {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + 'static + Send + Sync>> {
        use std::num::NonZeroU32;

        Ok(DieselFromSqlWrapper(Some(PgValue::new(
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

    fn get_raw_value<'a>(&'a self, index: usize) -> Option<PgValue<'a>> {
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
