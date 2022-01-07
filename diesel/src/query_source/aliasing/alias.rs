use super::{AliasSource, AliasedField, FieldAliasMapper};

use crate::associations::HasTable;
use crate::backend::Backend;
use crate::dsl::{Filter, Select};
use crate::expression::{Expression, SelectableExpression, ValidGrouping};
use crate::query_builder::nodes::StaticQueryFragment;
use crate::query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::query_dsl::methods::*;
use crate::query_dsl::QueryDsl;
use crate::query_source::{AppearsInFromClause, Column, Never, QuerySource, Table, TableNotEqual};
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, Default)]
/// Represents an alias within diesel's query builder
///
/// See [alias!] for more details.
pub struct Alias<S> {
    source: S,
}

impl<S: AliasSource> Alias<S> {
    /// Maps a single field of the source table in this alias
    pub fn field<F>(&self, field: F) -> AliasedField<S, F>
    where
        S: Clone,
        F: Column<Table = S::Table>,
    {
        AliasedField {
            _alias_source: self.source.clone(),
            _field: field,
        }
    }
    /// Maps multiple fields of the source table in this alias (takes in tuples)
    pub fn fields<Fields>(&self, fields: Fields) -> <Fields as FieldAliasMapper<S>>::Out
    where
        Fields: FieldAliasMapper<S>,
    {
        fields.map(self)
    }
}

impl<S> Alias<S> {
    #[doc(hidden)]
    /// May be used to create an alias. Used by the [`alias!`] macro.
    pub const fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> QueryId for Alias<S>
where
    Self: 'static,
    S: AliasSource,
    S::Table: Table,
{
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<S> QuerySource for Alias<S>
where
    S: AliasSource + Clone,
    S::Table: QuerySource + HasTable<Table = S::Table>,
    <S::Table as QuerySource>::DefaultSelection: FieldAliasMapper<S>,
    <<S::Table as QuerySource>::DefaultSelection as FieldAliasMapper<S>>::Out:
        SelectableExpression<Self>,
{
    type FromClause = Self;
    type DefaultSelection =
        <<S::Table as QuerySource>::DefaultSelection as FieldAliasMapper<S>>::Out;

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.fields(S::Table::table().default_selection())
    }
}

impl<S, DB> QueryFragment<DB> for Alias<S>
where
    S: AliasSource,
    DB: Backend,
    S::Table: StaticQueryFragment,
    <S::Table as StaticQueryFragment>::Component: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <S::Table as StaticQueryFragment>::STATIC_COMPONENT.walk_ast(pass.reborrow())?;
        pass.push_sql(" AS ");
        pass.push_identifier(S::NAME)?;
        Ok(())
    }
}

impl<S> AsQuery for Alias<S>
where
    S: AliasSource,
    S::Table: AsQuery,
    Self: QuerySource,
    <Self as QuerySource>::DefaultSelection: ValidGrouping<()>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<S: AliasSource> QueryDsl for Alias<S> {}

impl<S, Predicate> FilterDsl<Predicate> for Alias<S>
where
    S: AliasSource,
    Self: AsQuery,
    <Self as AsQuery>::Query: FilterDsl<Predicate>,
{
    type Output = Filter<<Self as AsQuery>::Query, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}

impl<S, Selection> SelectDsl<Selection> for Alias<S>
where
    Selection: Expression,
    Self: AsQuery,
    S: AliasSource,
    <Self as AsQuery>::Query: SelectDsl<Selection>,
{
    type Output = Select<<Self as AsQuery>::Query, Selection>;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}

impl<S, QS> AppearsInFromClause<QS> for Alias<S>
where
    S: AliasSource,
    S::Table: AliasAppearsInFromClause<S, QS>,
{
    type Count = <S::Table as AliasAppearsInFromClause<S, QS>>::Count;
}
#[doc(hidden)]
/// This trait is used to allow external crates to implement
/// `AppearsInFromClause<QS> for Alias<S>`
///
/// without running in conflicting impl issues
pub trait AliasAppearsInFromClause<S, QS> {
    /// Will be passed on to the `impl AppearsInFromClause<QS>`
    type Count;
}
#[doc(hidden)]
/// This trait is used to allow external crates to implement
/// `AppearsInFromClause<Alias<S2>> for Alias<S1>`
///
/// without running in conflicting impl issues
pub trait AliasAliasAppearsInFromClause<T2, S1, S2> {
    /// Will be passed on to the `impl AppearsInFromClause<QS>`
    type Count;
}
impl<T1, S1, S2> AliasAppearsInFromClause<S1, Alias<S2>> for T1
where
    S2: AliasSource,
    T1: AliasAliasAppearsInFromClause<S2::Table, S1, S2>,
{
    type Count = <T1 as AliasAliasAppearsInFromClause<S2::Table, S1, S2>>::Count;
}

// impl<S: AliasSource<Table=T1>> AppearsInFromClause<T2> for Alias<S>
// where T1 != T2
impl<T1, T2, S> AliasAppearsInFromClause<S, T2> for T1
where
    T1: TableNotEqual<T2> + Table,
    T2: Table,
    S: AliasSource<Table = T1>,
{
    type Count = Never;
}

// impl<S1, S2> AppearsInFromClause<Alias<S1>> for Alias<S2>
// where S1: AliasSource, S2: AliasSource, S1::Table != S2::Table
impl<T1, T2, S1, S2> AliasAliasAppearsInFromClause<T1, S2, S1> for T2
where
    T1: TableNotEqual<T2> + Table,
    T2: Table,
    S1: AliasSource<Table = T1>,
    S2: AliasSource<Table = T2>,
{
    type Count = Never;
}

impl<S> AppearsInFromClause<Alias<S>> for ()
where
    S: AliasSource,
{
    type Count = Never;
}
