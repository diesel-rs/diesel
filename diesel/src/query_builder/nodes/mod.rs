use crate::backend::Backend;
use crate::query_builder::*;
use crate::result::QueryResult;

#[derive(Debug, Copy, Clone)]
pub struct Identifier<'a>(pub &'a str);

impl<'a, DB: Backend> QueryFragment<DB> for Identifier<'a> {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(self.0)
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

impl<'a, T, U, DB> QueryFragment<DB> for InfixNode<'a, T, U>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.lhs.walk_ast(out.reborrow())?;
        out.push_sql(self.middle);
        self.rhs.walk_ast(out.reborrow())?;
        Ok(())
    }
}
