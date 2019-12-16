use super::*;
use crate::pg::PgQueryBuilder;

pub struct AsyncPgConnection {
    inner: PgConnection,
}

impl AsyncPgConnection {
    pub async fn establish(url: &str) -> ConnectionResult<Self> {
        let raw_connection = RawConnection::establish_async(url).await?;
        raw_connection.set_nonblocking()?;
        Ok(Self { inner: PgConnection::from_raw(raw_connection) })
    }

    pub async fn begin_test_transaction(&mut self) -> QueryResult<()> {
        self.execute_returning_count(crate::sql_query("BEGIN")).await?;
        Ok(())
    }

    pub(crate) async fn execute_returning_count<T>(&mut self, source: T) -> QueryResult<usize>
    where
        T: QueryFragment<Pg> + QueryId,
    {
        let (query, params) = self.prepare_query(source).await?;
        query.execute_async(&mut self.inner.raw_connection, &params)
            .await
            .map(|r| r.rows_affected())
    }

    pub(crate) async fn query_by_index<T, U>(&mut self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Pg> + QueryId,
        Pg: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Pg>,
    {
        let (query, params) = self.prepare_query(source.as_query()).await?;
        query
            .execute_async(&mut self.inner.raw_connection, &params)
            .await
            .and_then(|r| Cursor::new(r).collect())
    }

    // FIXME: Actually cache
    async fn prepare_query<T: QueryFragment<Pg> + QueryId>(
        &mut self,
        source: T,
    ) -> QueryResult<(Statement, Vec<Option<Vec<u8>>>)> {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        // FIXME: Make this error if we try to load anything dynamic
        source.collect_binds(&mut bind_collector, PgMetadataLookup::new(&self.inner))?;
        let binds = bind_collector.binds;
        let metadata = bind_collector.metadata;

        let mut query_builder = PgQueryBuilder::new();
        source.to_sql(&mut query_builder)?;
        let sql = query_builder.finish();

        let query = Statement::prepare_async(&mut self.inner.raw_connection, &sql, None, &metadata).await?;

        Ok((query, binds))
    }
}
