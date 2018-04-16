use backend::Backend;
use expression::{Expression, SelectableExpression};
use query_builder::*;
use query_source::QuerySource;
use sql_types::{NotNull, Nullable};

#[derive(Debug, Clone, Copy, QueryId)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct SelectClause<T>(pub(crate) T);
#[derive(Debug, Clone, Copy, QueryId)]
pub struct NullableSelectClause<T>(pub(crate) T);

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

impl<T, QS> SelectClauseExpression<QS> for NullableSelectClause<T>
where
    T: SelectClauseExpression<QS>,
    T::SelectClauseSqlType: NotNull,
{
    type SelectClauseSqlType = Nullable<T::SelectClauseSqlType>;
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

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for NullableSelectClause<T>
where
    DB: Backend,
    T: SelectClauseQueryFragment<QS, DB>,
{
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(source, pass)
    }
}

pub trait NotNullableSelectClause {}

impl NotNullableSelectClause for DefaultSelectClause {}
impl<T> NotNullableSelectClause for SelectClause<T> {}

pub trait BoxSelectClause<'a, DB: Backend, F> {
    fn box_select_clause(self, from: &F) -> Box<QueryFragment<DB> + 'a>;
}

impl<'a, DB: Backend, F> BoxSelectClause<'a, DB, F> for DefaultSelectClause
where
    F: QuerySource,
    F::DefaultSelection: QueryFragment<DB> + 'a,
{
    fn box_select_clause(self, from: &F) -> Box<QueryFragment<DB> + 'a> {
        Box::new(from.default_selection())
    }
}

impl<'a, DB: Backend, T, F> BoxSelectClause<'a, DB, F> for SelectClause<T>
where
    T: QueryFragment<DB> + 'a,
{
    fn box_select_clause(self, _from: &F) -> Box<QueryFragment<DB> + 'a> {
        Box::new(self.0)
    }
}

impl<'a, DB: Backend, T, F> BoxSelectClause<'a, DB, F> for NullableSelectClause<T>
where
    T: BoxSelectClause<'a, DB, F>,
{
    fn box_select_clause(self, from: &F) -> Box<QueryFragment<DB> + 'a> {
        self.0.box_select_clause(from)
    }
}
