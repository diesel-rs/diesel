use crate::UpdateAndFetchResults;
use crate::{AsyncConnection, AsyncConnectionCore, SimpleAsyncConnection, TransactionManager};
use diesel::associations::HasTable;
use diesel::connection::CacheSize;
use diesel::connection::Instrumentation;
use diesel::QueryResult;
use futures_util::future::BoxFuture;
use std::ops::DerefMut;

impl<C> SimpleAsyncConnection for C
where
    C: DerefMut + Send,
    C::Target: SimpleAsyncConnection + Send,
{
    async fn batch_execute(&mut self, query: &str) -> diesel::QueryResult<()> {
        let conn = self.deref_mut();
        conn.batch_execute(query).await
    }
}

impl<C> AsyncConnectionCore for C
where
    C: DerefMut + Send,
    C::Target: AsyncConnectionCore,
{
    type ExecuteFuture<'conn, 'query> =
        <C::Target as AsyncConnectionCore>::ExecuteFuture<'conn, 'query>;
    type LoadFuture<'conn, 'query> = <C::Target as AsyncConnectionCore>::LoadFuture<'conn, 'query>;
    type Stream<'conn, 'query> = <C::Target as AsyncConnectionCore>::Stream<'conn, 'query>;
    type Row<'conn, 'query> = <C::Target as AsyncConnectionCore>::Row<'conn, 'query>;

    type Backend = <C::Target as AsyncConnectionCore>::Backend;

    fn load<'conn, 'query, T>(&'conn mut self, source: T) -> Self::LoadFuture<'conn, 'query>
    where
        T: diesel::query_builder::AsQuery + 'query,
        T::Query: diesel::query_builder::QueryFragment<Self::Backend>
            + diesel::query_builder::QueryId
            + 'query,
    {
        let conn = self.deref_mut();
        conn.load(source)
    }

    fn execute_returning_count<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> Self::ExecuteFuture<'conn, 'query>
    where
        T: diesel::query_builder::QueryFragment<Self::Backend>
            + diesel::query_builder::QueryId
            + 'query,
    {
        let conn = self.deref_mut();
        conn.execute_returning_count(source)
    }
}

#[diagnostic::do_not_recommend]
impl<C> AsyncConnection for C
where
    C: DerefMut + Send,
    C::Target: AsyncConnection,
{
    type TransactionManager =
        PoolTransactionManager<<C::Target as AsyncConnection>::TransactionManager>;

    async fn establish(_database_url: &str) -> diesel::ConnectionResult<Self> {
        Err(diesel::result::ConnectionError::BadConnection(
            String::from("Cannot directly establish a pooled connection"),
        ))
    }

    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as crate::transaction_manager::TransactionManager<Self>>::TransactionStateData{
        let conn = self.deref_mut();
        conn.transaction_state()
    }

    async fn begin_test_transaction(&mut self) -> diesel::QueryResult<()> {
        self.deref_mut().begin_test_transaction().await
    }

    fn instrumentation(&mut self) -> &mut dyn Instrumentation {
        self.deref_mut().instrumentation()
    }

    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
        self.deref_mut().set_instrumentation(instrumentation);
    }

    fn set_prepared_statement_cache_size(&mut self, size: CacheSize) {
        self.deref_mut().set_prepared_statement_cache_size(size);
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct PoolTransactionManager<TM>(std::marker::PhantomData<TM>);

impl<C, TM> TransactionManager<C> for PoolTransactionManager<TM>
where
    C: DerefMut + Send,
    C::Target: AsyncConnection<TransactionManager = TM>,
    TM: TransactionManager<C::Target>,
{
    type TransactionStateData = TM::TransactionStateData;

    async fn begin_transaction(conn: &mut C) -> diesel::QueryResult<()> {
        TM::begin_transaction(&mut **conn).await
    }

    async fn rollback_transaction(conn: &mut C) -> diesel::QueryResult<()> {
        TM::rollback_transaction(&mut **conn).await
    }

    async fn commit_transaction(conn: &mut C) -> diesel::QueryResult<()> {
        TM::commit_transaction(&mut **conn).await
    }

    fn transaction_manager_status_mut(
        conn: &mut C,
    ) -> &mut diesel::connection::TransactionManagerStatus {
        TM::transaction_manager_status_mut(&mut **conn)
    }

    fn is_broken_transaction_manager(conn: &mut C) -> bool {
        TM::is_broken_transaction_manager(&mut **conn)
    }
}

impl<Changes, Output, Conn> UpdateAndFetchResults<Changes, Output> for Conn
where
    Conn: DerefMut + Send,
    Changes: diesel::prelude::Identifiable + HasTable + Send,
    Conn::Target: UpdateAndFetchResults<Changes, Output>,
{
    fn update_and_fetch<'conn, 'changes>(
        &'conn mut self,
        changeset: Changes,
    ) -> BoxFuture<'changes, QueryResult<Output>>
    where
        Changes: 'changes,
        'conn: 'changes,
        Self: 'changes,
    {
        self.deref_mut().update_and_fetch(changeset)
    }
}
