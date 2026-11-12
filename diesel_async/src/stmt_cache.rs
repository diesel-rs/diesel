use diesel::QueryResult;
use diesel::connection::statement_cache::{MaybeCached, StatementCallbackReturnType};
use futures_core::future::BoxFuture;
use futures_util::future::Either;
use futures_util::{FutureExt, TryFutureExt};
use std::future::{self, Future};

pub(crate) struct CallbackHelper<F>(pub(crate) F);

type PrepareFuture<'a, C, S> = Either<
    future::Ready<QueryResult<(MaybeCached<'a, S>, C)>>,
    BoxFuture<'a, QueryResult<(MaybeCached<'a, S>, C)>>,
>;

impl<S, F, C> StatementCallbackReturnType<S, C> for CallbackHelper<F>
where
    F: Future<Output = QueryResult<(S, C)>> + Send,
    S: 'static,
{
    type Return<'a> = PrepareFuture<'a, C, S>;

    fn from_error<'a>(e: diesel::result::Error) -> Self::Return<'a> {
        Either::Left(future::ready(Err(e)))
    }

    fn map_to_no_cache<'a>(self) -> Self::Return<'a>
    where
        Self: 'a,
    {
        Either::Right(
            self.0
                .map_ok(|(stmt, conn)| (MaybeCached::CannotCache(stmt), conn))
                .boxed(),
        )
    }

    fn map_to_cache(stmt: &mut S, conn: C) -> Self::Return<'_> {
        Either::Left(future::ready(Ok((MaybeCached::Cached(stmt), conn))))
    }

    fn register_cache<'a>(
        self,
        callback: impl FnOnce(S) -> &'a mut S + Send + 'a,
    ) -> Self::Return<'a>
    where
        Self: 'a,
    {
        Either::Right(
            self.0
                .map_ok(|(stmt, conn)| (MaybeCached::Cached(callback(stmt)), conn))
                .boxed(),
        )
    }
}

/// A driver's prepared statement as stored in diesel's statement cache
pub(crate) struct StatementHelper<S>(pub(crate) S);

pub(crate) struct QueryFragmentHelper {
    pub(crate) sql: String,
    pub(crate) safe_to_cache: bool,
    pub(crate) bind_count: usize,
}
