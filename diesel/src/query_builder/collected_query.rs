use super::{AstPass, MoveableBindCollector, Query, QueryFragment, QueryId};
use crate::backend::{Backend, DieselReserveSpecialization};
use crate::result::QueryResult;
use crate::sql_types::Untyped;

#[derive(Debug)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// A SQL query variant with already collected bind data which can be moved
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub struct CollectedQuery<T> {
    sql: String,
    safe_to_cache_prepared: bool,
    bind_data: T,
}

impl<T> CollectedQuery<T> {
    /// Builds a [CollectedQuery] with movable bind data
    pub fn new(sql: String, safe_to_cache_prepared: bool, bind_data: T) -> Self {
        Self {
            sql,
            safe_to_cache_prepared,
            bind_data,
        }
    }
}

impl<DB, T> QueryFragment<DB> for CollectedQuery<T>
where
    DB: Backend + DieselReserveSpecialization,
    for<'a> <DB as Backend>::BindCollector<'a>: MoveableBindCollector<DB, BindData = T>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        if !self.safe_to_cache_prepared {
            pass.unsafe_to_cache_prepared();
        }
        pass.push_sql(&self.sql);
        pass.push_bind_collector_data::<T>(&self.bind_data)
    }
}

impl<T> QueryId for CollectedQuery<T> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T> Query for CollectedQuery<T> {
    type SqlType = Untyped;
}
