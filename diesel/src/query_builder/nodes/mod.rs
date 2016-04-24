use backend::Backend;
use query_builder::{QueryBuilder, BuildQueryResult, QueryFragment};
use result::QueryResult;

#[derive(Debug, Copy, Clone)]
pub struct Identifier<'a>(pub &'a str);

impl<'a, DB: Backend> QueryFragment<DB> for Identifier<'a> {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_identifier(self.0)
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Join<T, U, V, W> {
    lhs: T,
    rhs: U,
    predicate: V,
    join_type: W,
}

impl<T, U, V, W> Join<T, U, V, W> {
    pub fn new(lhs: T, rhs: U, predicate: V, join_type: W) -> Self {
        Join {
            lhs: lhs,
            rhs: rhs,
            predicate: predicate,
            join_type: join_type,
        }
    }
}

pub trait CombinedJoin<Other> {
    type Output;

    fn combine_with(self, other: Other) -> Self::Output;
}

impl<T, U, UU, V, VV, W, WW> CombinedJoin<Join<U, UU, VV, WW>> for Join<T, U, V, W> {
    type Output = Join<
        Self,
        UU,
        VV,
        WW,
    >;

    fn combine_with(self, other: Join<U, UU, VV, WW>) -> Self::Output {
        Join::new(self, other.rhs, other.predicate, other.join_type)
    }
}

impl<T, U, V, W, DB> QueryFragment<DB> for Join<T, U, V, W> where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
    V: QueryFragment<DB>,
    W: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.lhs.to_sql(out));
        try!(self.join_type.to_sql(out));
        out.push_sql(" JOIN ");
        try!(self.rhs.to_sql(out));
        out.push_sql(" ON ");
        try!(self.predicate.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.lhs.collect_binds(out));
        try!(self.join_type.collect_binds(out));
        try!(self.rhs.collect_binds(out));
        try!(self.predicate.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.lhs.is_safe_to_cache_prepared() &&
            self.join_type.is_safe_to_cache_prepared() &&
            self.rhs.is_safe_to_cache_prepared() &&
            self.predicate.is_safe_to_cache_prepared()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InfixNode<'a, T, U> {
    lhs: T,
    rhs: U,
    middle: &'a str,
}

impl<'a, T, U> InfixNode<'a, T, U> {
    pub fn new(lhs: T, rhs: U, middle: &'a str) -> Self {
        InfixNode {
            lhs: lhs,
            rhs: rhs,
            middle: middle,
        }
    }
}

impl<'a, T, U, DB> QueryFragment<DB> for InfixNode<'a, T, U> where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.lhs.to_sql(out));
        out.push_sql(self.middle);
        try!(self.rhs.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.lhs.collect_binds(out));
        try!(self.rhs.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.lhs.is_safe_to_cache_prepared() &&
            self.rhs.is_safe_to_cache_prepared()
    }
}
