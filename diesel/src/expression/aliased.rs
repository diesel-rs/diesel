use backend::Backend;
use expression::{Expression, NonAggregate, SelectableExpression};
use query_builder::*;
use query_builder::nodes::{Identifier, InfixNode};
use query_source::*;
use result::QueryResult;

#[derive(Debug, Clone, Copy)]
pub struct Aliased<'a, Expr> {
    expr: Expr,
    alias: &'a str,
}

impl<'a, Expr> Aliased<'a, Expr> {
    pub fn new(expr: Expr, alias: &'a str) -> Self {
        Aliased {
            expr: expr,
            alias: alias,
        }
    }
}

impl<'a, T> Expression for Aliased<'a, T> where
    T: Expression,
{
    type SqlType = T::SqlType;
}

impl<'a, T, DB> QueryFragment<DB> for Aliased<'a, T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_identifier(self.alias)
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl<'a, T> QueryId for Aliased<'a, T> {
    type QueryId = ();

    fn has_static_query_id() -> bool {
        false
    }
}

// FIXME This is incorrect, should only be selectable from WithQuerySource
impl<'a, T, QS> SelectableExpression<QS> for Aliased<'a, T> where
    Aliased<'a, T>: Expression,
    T: SelectableExpression<QS>,
{
    type SqlTypeForSelect = T::SqlTypeForSelect;
}

impl<'a, T: Expression + Copy> QuerySource for Aliased<'a, T> {
    type FromClause = InfixNode<'static, T, Identifier<'a>>;

    fn from_clause(&self) -> Self::FromClause {
        InfixNode::new(self.expr, Identifier(self.alias), " ")
    }
}

impl<'a, T> NonAggregate for Aliased<'a, T> where Aliased<'a, T>: Expression {
}
