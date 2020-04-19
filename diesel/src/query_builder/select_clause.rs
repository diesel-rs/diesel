use std::marker::PhantomData;

use crate::backend::Backend;
use crate::deserialize::TableQueryable;
use crate::expression::{Expression, SelectableExpression};
use crate::query_builder::*;
use crate::query_source::QuerySource;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct SelectClause<T>(pub T);
#[derive(Debug, Clone, Copy)]
pub struct SelectByClause<T>(PhantomData<T>);
impl<T> QueryId for SelectByClause<T>
where
    T: TableQueryable,
    T::Columns: QueryId,
{
    type QueryId = <T::Columns as QueryId>::QueryId;
    const HAS_STATIC_QUERY_ID: bool = <T::Columns as QueryId>::HAS_STATIC_QUERY_ID;
}
impl<T> Default for SelectByClause<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub trait SelectClauseExpression<QS> {
    type Selection: SelectableExpression<QS>;
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectClause<T>
where
    T: SelectableExpression<QS>,
{
    type Selection = T;
    type SelectClauseSqlType = T::SqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectByClause<T>
where
    T: TableQueryable,
    T::Columns: SelectableExpression<QS>,
{
    type Selection = T::Columns;
    type SelectClauseSqlType = <T::Columns as Expression>::SqlType;
}

impl<QS> SelectClauseExpression<QS> for DefaultSelectClause
where
    QS: QuerySource,
{
    type Selection = QS::DefaultSelection;
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

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for SelectByClause<T>
where
    DB: Backend,
    T: TableQueryable,
    T::Columns: QueryFragment<DB>,
{
    fn walk_ast(&self, _: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        T::columns().walk_ast(pass)
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
