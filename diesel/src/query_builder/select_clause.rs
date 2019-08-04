use crate::backend::Backend;
use crate::expression::{Expression, SelectableExpression};
use crate::query_builder::*;
use crate::query_source::QuerySource;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct SelectClause<T>(pub T);

/// Specialised variant of `Expression` for select clause types
///
/// The difference to the normal `Expression` trait is the query source (`QS`)
/// generic type parameter. This allows to access the query source in generic code.
pub trait SelectClauseExpression<QS> {
    type Selection: SelectableExpression<QS>;
    /// SQL type of the select clause
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectClause<T>
where
    T: SelectableExpression<QS>,
{
    type Selection = T;
    type SelectClauseSqlType = T::SqlType;
}

impl<QS> SelectClauseExpression<QS> for DefaultSelectClause
where
    QS: QuerySource,
{
    type Selection = QS::DefaultSelection;
    type SelectClauseSqlType = <QS::DefaultSelection as Expression>::SqlType;
}

/// Specialised variant of `QueryFragment` for select clause types
///
/// The difference to the normal `QueryFragment` trait is the query source (`QS`)
/// generic type parameter.
pub trait SelectClauseQueryFragment<QS, DB: Backend> {
    /// Walk over this `SelectClauseQueryFragment` for all passes.
    ///
    /// This method is where the actual behavior of an select clause is implemented.
    /// This method will contain the behavior required for all possible AST
    /// passes. See [`AstPass`] for more details.
    ///
    /// [`AstPass`]: struct.AstPass.html
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
