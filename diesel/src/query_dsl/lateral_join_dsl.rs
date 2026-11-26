use crate::expression::AppearsOnTable;
use crate::query_builder::{
    AsQuery, BoxedCloneSelectStatement, BoxedSelectStatement, FromClause, SelectStatement,
};
use crate::query_dsl::InternalJoinDsl;
use crate::query_source::function_call::private::Lateral;
use crate::query_source::joins::OnClauseWrapper;
use crate::query_source::{Alias, FunctionCall, QueryRelation, QuerySource};

/// Constructs a join after checking the function arguments against its left source.
#[doc(hidden)]
pub trait InternalLateralJoinDsl<Rhs, Kind>: private::LateralSource {
    type Output;

    fn join_lateral(self, rhs: Rhs, kind: Kind) -> Self::Output;
}

impl<Left, R, F, On, Kind> InternalLateralJoinDsl<OnClauseWrapper<FunctionCall<R, F>, On>, Kind>
    for Left
where
    Left: private::LateralSource,
    F: AppearsOnTable<Left::Source>,
    Left::Query: InternalJoinDsl<FunctionCall<R, F, Lateral>, Kind, On>,
{
    type Output = <Left::Query as InternalJoinDsl<FunctionCall<R, F, Lateral>, Kind, On>>::Output;

    fn join_lateral(
        self,
        rhs: OnClauseWrapper<FunctionCall<R, F>, On>,
        kind: Kind,
    ) -> Self::Output {
        self.into_query().join(rhs.source.lateral(), kind, rhs.on)
    }
}
pub(crate) mod private {
    pub trait LateralSource {
        type Source;
        type Query;

        fn into_query(self) -> Self::Query;
    }
}

impl<T> private::LateralSource for T
where
    T: QueryRelation,
{
    type Source = T;
    type Query = T::Query;

    fn into_query(self) -> Self::Query {
        self.as_query()
    }
}

impl<S> private::LateralSource for Alias<S>
where
    S: crate::query_source::AliasSource,
    Self: AsQuery,
{
    type Source = Self;
    type Query = <Self as AsQuery>::Query;

    fn into_query(self) -> Self::Query {
        self.as_query()
    }
}

impl<R, F> private::LateralSource for FunctionCall<R, F>
where
    Self: AsQuery,
{
    type Source = Self;
    type Query = <Self as AsQuery>::Query;

    fn into_query(self) -> Self::Query {
        self.as_query()
    }
}

impl<F, S, D, W, O, LOf, G, H, LC> private::LateralSource
    for SelectStatement<FromClause<F>, S, D, W, O, LOf, G, H, LC>
where
    F: QuerySource,
{
    type Source = F;
    type Query = Self;

    fn into_query(self) -> Self::Query {
        self
    }
}

impl<ST, F, DB, GB> private::LateralSource for BoxedSelectStatement<'_, ST, FromClause<F>, DB, GB>
where
    F: QuerySource,
{
    type Source = F;
    type Query = Self;

    fn into_query(self) -> Self::Query {
        self
    }
}

impl<ST, F, DB, GB> private::LateralSource
    for BoxedCloneSelectStatement<'_, ST, FromClause<F>, DB, GB>
where
    F: QuerySource,
{
    type Source = F;
    type Query = Self;

    fn into_query(self) -> Self::Query {
        self
    }
}
