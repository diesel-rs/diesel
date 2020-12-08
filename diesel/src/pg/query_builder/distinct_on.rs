use crate::expression::SelectableExpression;
use crate::pg::Pg;
use crate::query_builder::{AstPass, QueryFragment, QueryId, SelectQuery, SelectStatement};
use crate::query_dsl::methods::DistinctOnDsl;
use crate::result::QueryResult;

/// Represents `DISTINCT ON (...)`
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DistinctOnClause<T>(pub(crate) T);

impl<T> QueryFragment<Pg> for DistinctOnClause<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("DISTINCT ON (");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<ST, F, S, D, W, O, LOf, G, Selection> DistinctOnDsl<Selection>
    for SelectStatement<F, S, D, W, O, LOf, G>
where
    Selection: SelectableExpression<F>,
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, DistinctOnClause<Selection>, W, O, LOf, G>: SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, DistinctOnClause<Selection>, W, O, LOf, G>;

    fn distinct_on(self, selection: Selection) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            DistinctOnClause(selection),
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}
