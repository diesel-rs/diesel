use backend::Backend;
use expression::*;
use query_builder::select_clause::{BoxSelectClause, SelectClauseExpression,
                                   SelectClauseQueryFragment};
use query_builder::*;
use query_source::Table;
use result::QueryResult;
use sql_types::IntoNullable;

#[derive(Debug, Copy, Clone, DieselNumericOps)]
pub struct Nullable<T>(T);

impl<T> Nullable<T> {
    pub fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T>
where
    T: Expression,
    <T as Expression>::SqlType: IntoNullable,
{
    type SqlType = <<T as Expression>::SqlType as IntoNullable>::Nullable;
}

impl<T, DB> QueryFragment<DB> for Nullable<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

/// Nullable can be used in where clauses everywhere, but can only be used in
/// select clauses for outer joins.
impl<T, QS> AppearsOnTable<QS> for Nullable<T>
where
    T: AppearsOnTable<QS>,
    Nullable<T>: Expression,
{
}

impl<T: QueryId> QueryId for Nullable<T> {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T> NonAggregate for Nullable<T>
where
    T: NonAggregate,
    Nullable<T>: Expression,
{
}

impl<T, Tab> SelectableExpression<Tab> for Nullable<T>
where
    Self: AppearsOnTable<Tab>,
    T: SelectableExpression<Tab>,
    Tab: Table,
{
}

impl<T, QS> SelectClauseExpression<QS> for Nullable<T>
where
    T: SelectClauseExpression<QS>,
    T::SelectClauseSqlType: ::sql_types::NotNull,
{
    type SelectClauseSqlType = ::sql_types::Nullable<T::SelectClauseSqlType>;
}

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for Nullable<T>
where
    T: SelectClauseQueryFragment<QS, DB>,
    DB: Backend,
{
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(source, pass)
    }
}

impl<'a, QS, DB, T> BoxSelectClause<'a, QS, DB> for Nullable<T>
where
    DB: Backend,
    T: BoxSelectClause<'a, QS, DB>,
{
    fn box_select_clause(self, qs: &QS) -> Box<QueryFragment<DB> + 'a> {
        self.0.box_select_clause(qs)
    }
}
