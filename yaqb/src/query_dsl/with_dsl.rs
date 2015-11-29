use expression::Expression;
use expression::aliased::Aliased;
use query_builder::*;
use query_source::QuerySource;

/// Adds an additional expression to the FROM clause. This is useful for things
/// like full text search, where you need to access the result of an expensive
/// computation for the where clause that shouldn't be redone for each row, such
/// as `plain_to_tsquery`. See
/// [`.aliased`](expression/expression_methods/global_expression_methods/trait.ExpressionMethods.html#method.aliased)
/// for more
pub trait WithDsl<'a, Expr> {
    type Output: AsQuery;

    fn with(self, expr: Aliased<'a, Expr>) -> Self::Output;
}

impl<'a, T, Expr> WithDsl<'a, Expr> for T where
    T: QuerySource + AsQuery,
    T::Query: WithDsl<'a, Expr>
{
    type Output = <T::Query as WithDsl<'a, Expr>>::Output;

    fn with(self, expr: Aliased<'a, Expr>) -> Self::Output {
        self.as_query().with(expr)
    }
}

#[doc(hidden)]
pub struct WithQuerySource<'a, Left, Right> {
    left: Left,
    right: Aliased<'a, Right>,
}

impl<'a, Left, Right> WithQuerySource<'a, Left, Right> {
    pub fn new(left: Left, right: Aliased<'a, Right>) -> Self {
        WithQuerySource {
            left: left,
            right: right,
        }
    }
}

impl<'a, Left, Right> QuerySource for WithQuerySource<'a, Left, Right> where
    Left: QuerySource,
    Aliased<'a, Right>: QuerySource + Expression,
{
    fn from_clause(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(self.left.from_clause(out));
        out.push_sql(", ");
        self.right.from_clause(out)
    }
}
