use expression::SelectableExpression;
use super::{Query, QueryFragment, QueryBuilder, BuildQueryResult};

pub struct BareSelectStatement<T> {
    expression: T,
}

impl<T> BareSelectStatement<T> {
    pub fn new(expression: T) -> Self {
        BareSelectStatement {
            expression: expression,
        }
    }
}

impl<T> QueryFragment for BareSelectStatement<T> where
    T: SelectableExpression<()>,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        self.expression.to_sql(out)
    }
}

impl<T> Query for BareSelectStatement<T> where
    T: SelectableExpression<()>,
{
    type SqlType = T::SqlType;
}
