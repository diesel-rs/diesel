//! Implements all the traits related to being able to join from/to aliases

use super::field_alias_mapper::FieldAliasMapper;
use super::{Alias, AliasSource, AliasedField};

use crate::expression::{AppearsOnTable, SelectableExpression};
use crate::query_builder::AsQuery;
use crate::query_dsl::InternalJoinDsl;
use crate::query_source::joins::ToInnerJoin;
use crate::query_source::joins::{
    AppendSelection, Inner, Join, JoinOn, JoinTo, LeftOuter, OnClauseWrapper,
};
use crate::query_source::{AppearsInFromClause, Never, Pick, QuerySource, Table};

impl<T, S> JoinTo<T> for Alias<S>
where
    T: Table,
    S: AliasSource + Default,
    S::Target: JoinTo<T>,
    <S::Target as JoinTo<T>>::OnClause: FieldAliasMapper<S>,
{
    type FromClause = <S::Target as JoinTo<T>>::FromClause;
    type OnClause = <<S::Target as JoinTo<T>>::OnClause as FieldAliasMapper<S>>::Out;

    fn join_target(rhs: T) -> (Self::FromClause, Self::OnClause) {
        let (from_clause, on_clause) = <S::Target as JoinTo<T>>::join_target(rhs);
        (from_clause, Self::default().fields(on_clause))
    }
}

impl<S2, S> JoinTo<Alias<S2>> for Alias<S>
where
    S2: AliasSource,
    S: AliasSource + Default,
    S::Target: JoinTo<Alias<S2>>,
    <S::Target as JoinTo<Alias<S2>>>::OnClause: FieldAliasMapper<S>,
{
    type FromClause = <S::Target as JoinTo<Alias<S2>>>::FromClause;
    type OnClause = <<S::Target as JoinTo<Alias<S2>>>::OnClause as FieldAliasMapper<S>>::Out;

    fn join_target(rhs: Alias<S2>) -> (Self::FromClause, Self::OnClause) {
        let (from_clause, on_clause) = <S::Target as JoinTo<Alias<S2>>>::join_target(rhs);
        (from_clause, Self::default().fields(on_clause))
    }
}

impl<S, Rhs, On> JoinTo<OnClauseWrapper<Rhs, On>> for Alias<S> {
    type FromClause = Rhs;
    type OnClause = On;

    fn join_target(rhs: OnClauseWrapper<Rhs, On>) -> (Self::FromClause, Self::OnClause) {
        (rhs.source, rhs.on)
    }
}

impl<S: AliasSource> ToInnerJoin for Alias<S> {
    type InnerJoin = Self;
}

impl<S, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for Alias<S>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <<Self as AsQuery>::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

impl<Left, Right, S, C> SelectableExpression<Join<Left, Right, LeftOuter>> for AliasedField<S, C>
where
    Self: AppearsOnTable<Join<Left, Right, LeftOuter>>,
    Self: SelectableExpression<Left>,
    Left: QuerySource,
    Right: AppearsInFromClause<Alias<S>, Count = Never> + QuerySource,
{
}

impl<Left, Right, S, C> SelectableExpression<Join<Left, Right, Inner>> for AliasedField<S, C>
where
    Self: AppearsOnTable<Join<Left, Right, Inner>>,
    Left: AppearsInFromClause<Alias<S>> + QuerySource,
    Right: AppearsInFromClause<Alias<S>> + QuerySource,
    (Left::Count, Right::Count): Pick<Left, Right>,
    Self: SelectableExpression<<(Left::Count, Right::Count) as Pick<Left, Right>>::Selection>,
{
}

// FIXME: Remove this when overlapping marker traits are stable
impl<Join, On, S, C> SelectableExpression<JoinOn<Join, On>> for AliasedField<S, C> where
    Self: SelectableExpression<Join> + AppearsOnTable<JoinOn<Join, On>>
{
}

impl<S, Selection> AppendSelection<Selection> for Alias<S>
where
    Self: QuerySource,
{
    type Output = (<Self as QuerySource>::DefaultSelection, Selection);

    fn append_selection(&self, selection: Selection) -> Self::Output {
        (self.default_selection(), selection)
    }
}
