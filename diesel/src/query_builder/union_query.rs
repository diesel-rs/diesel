use backend::Backend;
use result::QueryResult;
use super::{Query, CombinableQuery, QueryBuilder, QueryFragment, BuildQueryResult};

#[derive(Debug)]
pub struct UnionQuery<L, R> {
    left: L,
    right: R,
    all: bool,
}

impl<L, R> UnionQuery<L, R> {
    pub fn new(left: L, right: R, all: bool) -> Self {
        UnionQuery {
            left: left,
            right: right,
            all: all,
        }
    }
}

impl<L, R> Query for UnionQuery<L, R> where
    L: CombinableQuery,
    R: CombinableQuery<SqlType=L::SqlType>,
{
    type SqlType = <L as Query>::SqlType;
}

impl<L, R> CombinableQuery for UnionQuery<L, R> where
    UnionQuery<L, R>: Query,
{
}

impl<L, R, DB> QueryFragment<DB> for UnionQuery<L, R> where
    DB: Backend,
    L: QueryFragment<DB>,
    R: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.left.to_sql(out));
        if self.all {
            out.push_sql(" UNION ALL ");
        } else {
            out.push_sql(" UNION ");
        }
        try!(self.right.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.left.collect_binds(out));
        try!(self.right.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.left.is_safe_to_cache_prepared() && self.right.is_safe_to_cache_prepared()
    }
}

impl_query_id!(UnionQuery<L, R>);
