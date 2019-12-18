#![warn(missing_docs)]

use super::*;
use associations::HasTable;
use backend::Backend;
use dsl::{Filter, Select};
use expression::{AppearsOnTable, Expression, NonAggregate, SelectableExpression};
use query_builder::{AsQuery, AstPass, QueryFragment, QueryId, SelectStatement};
use query_dsl::methods::*;
use query_dsl::QueryDsl;
use query_source::{AppearsInFromClause, Column, Never, Once, QuerySource, Table};
use result::QueryResult;
use std::marker::PhantomData;

#[derive(Debug, Copy)]
pub struct Alias<T, F>(PhantomData<(T, F)>);

#[derive(Debug)]
pub struct AliasedField<A, F>(A, PhantomData<F>);

pub trait Named {
    const NAME: &'static str;
}

impl<T, F> Clone for Alias<T, F> {
    fn clone(&self) -> Self {
        Alias::new()
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
            impl<_T, _F, $($T,)*> FieldAliasMapper<Alias<_T, _F>> for ($($T,)*) {
                type Out = ($(AliasedField<Alias<_T, _F>, $T>,)*);

                fn map(self) -> Self::Out {
                    (
                        $(AliasedField(Alias::new(), PhantomData::<$T>),)*
                    )
                }
            }
        )*
    }
}

__diesel_for_each_tuple!(field_alias_mapper);

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
    T: Table + QuerySource + HasTable<Table = T>,
    T::FromClause: QueryFragment<DB>,
    F: Named,
{
    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        T::from_clause(&T::table()).walk_ast(pass.reborrow())?;
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
    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
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
{
}

impl<T, F, C> NonAggregate for AliasedField<Alias<T, F>, C>
where
    T: Table,
    C: Column<Table = T>,
{
}

impl<T, F> AsQuery for Alias<T, F>
where
    T: AsQuery + Table + HasTable<Table = T>,
    Self: QuerySource,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<Self>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<T, F> AppearsInFromClause<Alias<T, F>> for Alias<T, F>
where
    T: Table,
{
    type Count = Once;
}

impl<T, F> AppearsInFromClause<Alias<T, F>> for ()
where
    T: Table,
{
    type Count = Never;
}

// impl<T, F> AppearsInFromClause<Alias<T, F>> for T
// where
//     T: Table,
// {
//     type Count = Never;
// }

// impl<F> AppearsInFromClause<dependent_operations::table> for Alias<operation_states::table, F> {
//     type Count = Never;
// }

// impl<F> AppearsInFromClause<Alias<operation_states::table, F>> for dependent_operations::table {
//     type Count = Never;
// }

// impl<F> AppearsInFromClause<Alias<operation_states::table, F>> for operation_states::table {
//     type Count = Never;
// }

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

/// TODO
#[macro_export]
macro_rules! alias {
    ($table: ident as $alias: ident) => {{
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy)]
        struct $alias;

        impl $crate::query_source::Named for $alias {
            const NAME: &'static str = stringify!($alias);
        }

        $crate::query_source::Alias::<$table::table, $alias>::new()
    }};
}
