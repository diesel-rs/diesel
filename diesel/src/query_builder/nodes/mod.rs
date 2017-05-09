use backend::Backend;
use query_builder::*;
use result::QueryResult;

#[derive(Debug, Copy, Clone)]
pub struct Identifier<'a>(pub &'a str);

impl<'a, DB: Backend> QueryFragment<DB> for Identifier<'a> {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_identifier(self.0)
    }

    fn walk_ast(&self, _: &mut AstPass<DB>) -> QueryResult<()> {
        Ok(())
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

    fn walk_ast(&self, pass: &mut AstPass<DB>) -> QueryResult<()> {
        self.lhs.walk_ast(pass)?;
        self.join_type.walk_ast(pass)?;
        self.rhs.walk_ast(pass)?;
        self.predicate.walk_ast(pass)?;
        Ok(())
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

    fn walk_ast(&self, pass: &mut AstPass<DB>) -> QueryResult<()> {
        self.lhs.walk_ast(pass)?;
        self.rhs.walk_ast(pass)?;
        Ok(())
    }
}
