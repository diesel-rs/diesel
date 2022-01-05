#![warn(missing_docs)]

use super::*;
use crate::associations::HasTable;
use crate::backend::Backend;
use crate::dsl::{Filter, Select};
use crate::expression::{
    is_aggregate, AppearsOnTable, Expression, SelectableExpression, ValidGrouping,
};
use crate::query_builder::nodes::StaticQueryFragment;
use crate::query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::query_dsl::join_dsl::InternalJoinDsl;
use crate::query_dsl::methods::*;
use crate::query_dsl::QueryDsl;
use crate::query_source::joins::ToInnerJoin;
use crate::query_source::joins::{
    AppendSelection, Inner, Join, JoinOn, LeftOuter, OnClauseWrapper,
};
use crate::query_source::{AppearsInFromClause, Column, Never, Once, QuerySource, Table};
use crate::result::QueryResult;
use std::marker::PhantomData;

#[derive(Debug, Copy)]
pub struct Alias<T, F>(PhantomData<(T, F)>);

#[derive(Debug)]
pub struct AliasedField<A, F>(A, PhantomData<F>);

pub trait Named {
    const NAME: &'static str;
}

pub trait AliasNotEqualHelper<Table2, Alias1, Alias2> {
    type Count;
}

impl<T, F> Clone for Alias<T, F> {
    fn clone(&self) -> Self {
        Alias::new()
    }
}

impl<T, F, C> Clone for AliasedField<Alias<T, F>, C> {
    fn clone(&self) -> Self {
        AliasedField(self.0.clone(), PhantomData)
    }
}

impl<T, F> QueryId for Alias<T, F> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T, F> Alias<T, F> {
    pub fn new() -> Self {
        Alias(PhantomData)
    }
}

impl<T, F> Alias<T, F>
where
    T: Table,
{
    pub fn field<C>(&self, _: C) -> AliasedField<Self, C>
    where
        C: Column<Table = T>,
    {
        AliasedField(Alias::new(), PhantomData)
    }
    pub fn fields<Fields>(&self, fields: Fields) -> <Fields as FieldAliasMapper<Self>>::Out
    where
        Fields: FieldAliasMapper<Self>,
    {
        fields.map()
    }
}

impl<QS, T, F, C> AppearsOnTable<QS> for AliasedField<Alias<T, F>, C>
where
    QS: AppearsInFromClause<Alias<T, F>, Count = Once>,
    T: Table,
    C: Column<Table = T>,
{
}

impl<T, F, C> QueryId for AliasedField<Alias<T, F>, C>
where
    T: Table + 'static,
    C: Column<Table = T> + 'static,
    F: 'static,
{
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

#[doc(hidden)]
pub trait FieldAliasMapper<A> {
    type Out;

    fn map(self) -> Self::Out;
}

impl<T, F, C> FieldAliasMapper<Alias<T, F>> for C
where
    C: Column<Table = T>,
    T: Table,
{
    type Out = AliasedField<Alias<T, F>, C>;

    fn map(self) -> Self::Out {
        AliasedField(Alias::new(), PhantomData)
    }
}

macro_rules! field_alias_mapper {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<_T, _F, $($T,)*> FieldAliasMapper<Alias<_T, _F>> for ($($T,)*)
            where
                $($T: FieldAliasMapper<Alias<_T, _F>>,)*
            {
                type Out = ($(<$T as FieldAliasMapper<Alias<_T, _F>>>::Out,)*);

                fn map(self) -> Self::Out {
                    (
                        $(self.$idx.map(),)*
                    )
                }
            }
        )*
    }
}

diesel_derives::__diesel_for_each_tuple!(field_alias_mapper);

impl<T, F> QuerySource for Alias<T, F>
where
    T: Table + QuerySource + HasTable<Table = T>,
    T::DefaultSelection: FieldAliasMapper<Self>,
    <T::DefaultSelection as FieldAliasMapper<Self>>::Out: SelectableExpression<Self>,
{
    type FromClause = Self;
    type DefaultSelection = <T::DefaultSelection as FieldAliasMapper<Self>>::Out;

    fn from_clause(&self) -> Self::FromClause {
        Alias::new()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        T::default_selection(&T::table()).map()
    }
}

impl<T, F, DB> QueryFragment<DB> for Alias<T, F>
where
    DB: Backend,
    T: Table + StaticQueryFragment,
    T::Component: QueryFragment<DB>,
    F: Named,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        T::STATIC_COMPONENT.walk_ast(pass.reborrow())?;
        pass.push_sql(" AS ");
        pass.push_identifier(F::NAME)?;
        Ok(())
    }
}

impl<T, F, C, DB> QueryFragment<DB> for AliasedField<Alias<T, F>, C>
where
    DB: Backend,
    T: Table,
    C: Column<Table = T>,
    F: Named,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_identifier(F::NAME)?;
        pass.push_sql(".");
        pass.push_identifier(C::NAME)?;
        Ok(())
    }
}

impl<T, F, C> Expression for AliasedField<Alias<T, F>, C>
where
    T: Table,
    C: Column<Table = T> + Expression,
{
    type SqlType = C::SqlType;
}

impl<T, F, C> SelectableExpression<Alias<T, F>> for AliasedField<Alias<T, F>, C>
where
    T: Table,
    C: Column<Table = T>,
    Self: AppearsOnTable<Alias<T, F>>,
{
}

impl<T, F, C> ValidGrouping<()> for AliasedField<Alias<T, F>, C>
where
    T: Table,
    C: Column<Table = T>,
{
    type IsAggregate = is_aggregate::No;
}
impl<T, F, C> ValidGrouping<AliasedField<Alias<T, F>, C>> for AliasedField<Alias<T, F>, C>
where
    T: Table,
    C: Column<Table = T>,
{
    type IsAggregate = is_aggregate::Yes;
}

impl<T, F> AsQuery for Alias<T, F>
where
    T: AsQuery + Table + HasTable<Table = T>,
    Self: QuerySource,
    <Self as QuerySource>::DefaultSelection: ValidGrouping<()>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<T1, T2, F1, F2> AppearsInFromClause<Alias<T1, F1>> for Alias<T2, F2>
where
    T2: AliasNotEqualHelper<T1, F2, F1>,
{
    type Count = T2::Count;
}

impl<T, F> AppearsInFromClause<Alias<T, F>> for ()
where
    T: Table,
{
    type Count = Never;
}

impl<T, F> QueryDsl for Alias<T, F> where T: Table {}

impl<T, F, Predicate> FilterDsl<Predicate> for Alias<T, F>
where
    T: Table,
    Self: AsQuery,
    <Self as AsQuery>::Query: FilterDsl<Predicate>,
{
    type Output = Filter<<Self as AsQuery>::Query, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}

impl<T, F, Selection> SelectDsl<Selection> for Alias<T, F>
where
    Selection: Expression,
    Self: AsQuery,
    T: Table,
    <Self as AsQuery>::Query: SelectDsl<Selection>,
{
    type Output = Select<<Self as AsQuery>::Query, Selection>;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}

#[macro_export]
macro_rules! __internal_alias_helper {
    (
        $left_table: ident as $left_alias: ident,
        $right_table: ident as $right_alias: ident,
        $($table: ident as $alias: ident,)*
    ) => {
        static_cond!{if $left_table == $right_table {
            impl $crate::query_source::AliasNotEqualHelper<$left_table::table, $right_alias, $left_alias> for $right_table::table {
                type Count = $crate::query_source::Never;
            }

            impl $crate::query_source::AliasNotEqualHelper<$right_table::table, $left_alias, $right_alias> for $left_table::table {
                type Count = $crate::query_source::Never;
            }
        }}

        __internal_alias_helper!($left_table as $left_alias, $($table as $alias,)*);
        __internal_alias_helper!($right_table as $right_alias, $($table as $alias,)*);
    };

    ($table: ident as $alias: ident,) => {}
}

/// TODO
#[macro_export]
macro_rules! alias {
    ($($table: ident as $alias: ident),* $(,)?) => {{
        $(
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy)]
            struct $alias;

            impl $crate::query_source::Named for $alias {
                const NAME: &'static str = stringify!($alias);
            }

            impl
                $crate::query_source::AppearsInFromClause<
                $crate::query_source::Alias<$table::table, $alias>,
            > for $table::table
            {
                type Count = $crate::query_source::Never;
            }

            impl $crate::query_source::AppearsInFromClause<$table::table>
                for $crate::query_source::Alias<$table::table, $alias>
            {
                type Count = $crate::query_source::Never;
            }

            impl $crate::query_source::AliasNotEqualHelper<$table::table, $alias, $alias> for $table::table {
                type Count = $crate::query_source::Once;
            }
        )*
        __internal_alias_helper!($($table as $alias,)*);
        ($($crate::query_source::Alias::<$table::table, $alias>::new()),*)
    }};
}

impl<T: Table, F> ToInnerJoin for Alias<T, F> {
    type InnerJoin = Self;
}

impl<T, Rhs, Kind, On, F> InternalJoinDsl<Rhs, Kind, On> for Alias<T, F>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <<Self as AsQuery>::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

impl<Left, Right, T, F, C> SelectableExpression<Join<Left, Right, LeftOuter>>
    for AliasedField<Alias<T, F>, C>
where
    Self: AppearsOnTable<Join<Left, Right, LeftOuter>>,
    Self: SelectableExpression<Left>,
    Left: QuerySource,
    Right: AppearsInFromClause<Alias<T, F>, Count = Never> + QuerySource,
{
}

impl<Left, Right, T, F, C> SelectableExpression<Join<Left, Right, Inner>>
    for AliasedField<Alias<T, F>, C>
where
    Self: AppearsOnTable<Join<Left, Right, Inner>>,
    Left: AppearsInFromClause<Alias<T, F>> + QuerySource,
    Right: AppearsInFromClause<Alias<T, F>> + QuerySource,
    (Left::Count, Right::Count): Pick<Left, Right>,
    Self: SelectableExpression<<(Left::Count, Right::Count) as Pick<Left, Right>>::Selection>,
{
}

// FIXME: Remove this when overlapping marker traits are stable
impl<Join, On, T, F, C> SelectableExpression<JoinOn<Join, On>> for AliasedField<Alias<T, F>, C> where
    Self: SelectableExpression<Join> + AppearsOnTable<JoinOn<Join, On>>
{
}

// FIXME: Remove this when overlapping marker traits are stable
impl<From, T, F, C> SelectableExpression<SelectStatement<FromClause<From>>>
    for AliasedField<Alias<T, F>, C>
where
    Self: SelectableExpression<From> + AppearsOnTable<SelectStatement<FromClause<From>>>,
    From: QuerySource,
{
}

impl<T, Selection, F> AppendSelection<Selection> for Alias<T, F>
where
    T: Table,
    Self: QuerySource,
{
    type Output = (<Self as QuerySource>::DefaultSelection, Selection);

    fn append_selection(&self, selection: Selection) -> Self::Output {
        (self.default_selection(), selection)
    }
}

impl<Lhs, Rhs, On, F> JoinTo<OnClauseWrapper<Rhs, On>> for Alias<Lhs, F> {
    type FromClause = Rhs;
    type OnClause = On;

    fn join_target(rhs: OnClauseWrapper<Rhs, On>) -> (Self::FromClause, Self::OnClause) {
        (rhs.source, rhs.on)
    }
}
