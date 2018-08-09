use backend::Backend;
use expression::{Expression, SelectableExpression};
use query_builder::*;
use query_source::QuerySource;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct SelectClause<T>(pub T);

pub trait SelectClauseExpression<QS> {
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectClause<T>
where
    T: SelectableExpression<QS>,
{
    type SelectClauseSqlType = T::SqlType;
}

impl<QS> SelectClauseExpression<QS> for DefaultSelectClause
where
    QS: QuerySource,
{
    type SelectClauseSqlType = <QS::DefaultSelection as Expression>::SqlType;
}

pub trait SelectClauseQueryFragment<QS, DB: Backend> {
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()>;
}

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for SelectClause<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, _: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<QS, DB> SelectClauseQueryFragment<QS, DB> for DefaultSelectClause
where
    DB: Backend,
    QS: QuerySource,
    QS::DefaultSelection: QueryFragment<DB>,
{
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        source.default_selection().walk_ast(pass)
    }
}

#[doc(hidden)]
pub trait BoxSelectClause<'a, QS, DB: Backend> {
    fn box_select_clause(self, qs: &QS) -> Box<QueryFragment<DB> + 'a>;
}

impl<'a, QS, DB, T> BoxSelectClause<'a, QS, DB> for SelectClause<T>
where
    DB: Backend,
    T: QueryFragment<DB> + SelectableExpression<QS> + 'a,
{
    fn box_select_clause(self, _: &QS) -> Box<QueryFragment<DB> + 'a> {
        Box::new(self.0)
    }
}

impl<'a, QS, DB> BoxSelectClause<'a, QS, DB> for DefaultSelectClause
where
    DB: Backend,
    QS: QuerySource,
    QS::DefaultSelection: QueryFragment<DB> + 'a,
{
    fn box_select_clause(self, qs: &QS) -> Box<QueryFragment<DB> + 'a> {
        Box::new(qs.default_selection())
    }
}
