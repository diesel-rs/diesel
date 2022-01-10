use super::{Alias, AliasSource};

use crate::dsl;
use crate::expression::{
    is_aggregate, AppearsOnTable, AsExpression, Expression, SelectableExpression, ValidGrouping,
};
use crate::expression_methods::{EqAll, ExpressionMethods};
use crate::query_builder::{AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::sql_types;

use crate::backend::Backend;
use crate::query_source::{AppearsInFromClause, Column, Once, QuerySource};
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, Default)]
/// Represents an aliased field (column) within diesel's query builder
///
/// See [alias!] for more details.
pub struct AliasedField<S, F> {
    pub(super) _alias_source: S,
    pub(super) _field: F,
}

impl<S, C> QueryId for AliasedField<S, C>
where
    S: AliasSource + 'static,
    S::Table: 'static,
    C: Column<Table = S::Table> + 'static,
{
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<QS, S, C> AppearsOnTable<QS> for AliasedField<S, C>
where
    S: AliasSource,
    QS: AppearsInFromClause<Alias<S>, Count = Once>,
    C: Column<Table = S::Table>,
{
}

impl<S, C, DB> QueryFragment<DB> for AliasedField<S, C>
where
    S: AliasSource,
    DB: Backend,
    C: Column<Table = S::Table>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_identifier(S::NAME)?;
        pass.push_sql(".");
        pass.push_identifier(C::NAME)?;
        Ok(())
    }
}

impl<S, C> Expression for AliasedField<S, C>
where
    S: AliasSource,
    C: Column<Table = S::Table> + Expression,
{
    type SqlType = C::SqlType;
}

impl<S, C> SelectableExpression<Alias<S>> for AliasedField<S, C>
where
    S: AliasSource,
    C: Column<Table = S::Table>,
    Self: AppearsOnTable<Alias<S>>,
{
}

impl<S, C> ValidGrouping<()> for AliasedField<S, C>
where
    S: AliasSource,
    C: Column<Table = S::Table>,
{
    type IsAggregate = is_aggregate::No;
}
impl<S, C> ValidGrouping<AliasedField<S, C>> for AliasedField<S, C>
where
    S: AliasSource,
    C: Column<Table = S::Table>,
{
    type IsAggregate = is_aggregate::Yes;
}

// FIXME: Remove this when overlapping marker traits are stable
impl<From, S, C> SelectableExpression<SelectStatement<FromClause<From>>> for AliasedField<S, C>
where
    Self: SelectableExpression<From> + AppearsOnTable<SelectStatement<FromClause<From>>>,
    From: QuerySource,
{
}

impl<S, C, T> EqAll<T> for AliasedField<S, C>
where
    S: AliasSource,
    C: Column<Table = S::Table>,
    Self: ExpressionMethods,
    <Self as Expression>::SqlType: sql_types::SqlType,
    T: AsExpression<<Self as Expression>::SqlType>,
    dsl::Eq<Self, T>: Expression<SqlType = sql_types::Bool>,
{
    type Output = dsl::Eq<Self, T>;
    fn eq_all(self, rhs: T) -> Self::Output {
        self.eq(rhs)
    }
}
