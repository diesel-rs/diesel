use backend::Backend;
use expression::{Expression, SelectableExpression};
use query_builder::*;
use query_source::QuerySource;

#[derive(Debug, Clone, Copy)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy)]
pub struct SelectClause<T>(pub T);

impl_query_id!(DefaultSelectClause);
impl_query_id!(SelectClause<T>);

pub trait SelectClauseExpression<QS> {
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectClause<T> where
    T: SelectableExpression<QS>,
{
    type SelectClauseSqlType = T::SqlType;
}

impl<QS> SelectClauseExpression<QS> for DefaultSelectClause where
    QS: QuerySource,
{
    type SelectClauseSqlType = <QS::DefaultSelection as Expression>::SqlType;
}

pub trait SelectClauseQueryFragment<QS, DB: Backend> {
    fn to_sql(&self, source: &QS, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()>;
}

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for SelectClause<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, _: &QS, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.0.to_sql(out)
    }

    fn walk_ast(&self, _: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<QS, DB> SelectClauseQueryFragment<QS, DB> for DefaultSelectClause where
    DB: Backend,
    QS: QuerySource,
    QS::DefaultSelection: QueryFragment<DB>,
{
    fn to_sql(&self, source: &QS, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        source.default_selection().to_sql(out)
    }

    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        source.default_selection().walk_ast(pass)
    }
}
