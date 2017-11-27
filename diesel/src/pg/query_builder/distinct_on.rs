use pg::Pg;
use query_dsl::DistinctOnDsl;
use query_builder::{AstPass, QueryFragment, SelectStatement};
use result::QueryResult;
use expression::{Expression, SelectableExpression};

#[derive(Debug, Clone, Copy)]
pub struct DistinctOnClause<T>(pub(crate) T);

impl<T> QueryFragment<Pg> for DistinctOnClause<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("DISTINCT ON(");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_query_id!(DistinctOnClause<T>);

impl<ST, F, S, D, W, O, L, Of, G, Selection> DistinctOnDsl<Selection, F>
    for SelectStatement<F, S, D, W, O, L, Of, G>
where
    Selection: SelectableExpression<F>,
    Self: Expression<SqlType = ST>,
    SelectStatement<F, S, DistinctOnClause<Selection>, W, O, L, Of, G>: Expression<SqlType = ST>,
{
    type Output = SelectStatement<F, S, DistinctOnClause<Selection>, W, O, L, Of, G>;

    fn distinct_on(self, selection: Selection) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            DistinctOnClause(selection),
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.for_update,
        )
    }
}
