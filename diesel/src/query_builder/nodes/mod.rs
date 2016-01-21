use query_builder::{QueryBuilder, BuildQueryResult, QueryFragment};

pub struct Identifier<'a>(pub &'a str);

impl<'a> QueryFragment for Identifier<'a> {
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_identifier(self.0)
    }
}

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

impl<T, U, V, W> QueryFragment for Join<T, U, V, W> where
    T: QueryFragment,
    U: QueryFragment,
    V: QueryFragment,
    W: QueryFragment,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(self.lhs.to_sql(out));
        try!(self.join_type.to_sql(out));
        out.push_sql(" JOIN ");
        try!(self.rhs.to_sql(out));
        out.push_sql(" ON ");
        try!(self.predicate.to_sql(out));
        Ok(())
    }
}

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

impl<'a, T, U> QueryFragment for InfixNode<'a, T, U> where
    T: QueryFragment,
    U: QueryFragment,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(self.lhs.to_sql(out));
        out.push_sql(self.middle);
        try!(self.rhs.to_sql(out));
        Ok(())
    }
}
