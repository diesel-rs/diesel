use super::field_alias_mapper::FieldAliasMapper;
use super::{AliasSource, AliasedField};

use crate::backend::{sql_dialect, Backend};
use crate::expression::{Expression, SelectableExpression, ValidGrouping};
use crate::helper_types::AliasedFields;
use crate::query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::query_source::{AppearsInFromClause, Column, Never, QuerySource, Table, TableNotEqual};
use crate::result::QueryResult;

use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, Default)]
/// Represents an alias within diesel's query builder
///
/// See [`alias!`](crate::alias) for more details.
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(source)
)]
pub struct Alias<S> {
    /// The inner alias definition
    pub(crate) source: S,
}

impl<S: AliasSource> Alias<S> {
    /// Maps a single field of the source table in this alias
    pub fn field<F>(&self, field: F) -> AliasedField<S, F>
    where
        F: Column<Table = S::Target>,
    {
        AliasedField {
            _alias_source: PhantomData,
            _field: field,
        }
    }
    /// Maps multiple fields of the source table in this alias
    /// (takes in tuples and some expressions)
    pub fn fields<Fields>(&self, fields: Fields) -> AliasedFields<S, Fields>
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
    S::Target: QueryId,
{
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = <S::Target as QueryId>::HAS_STATIC_QUERY_ID;
}

impl<S> QuerySource for Alias<S>
where
    Self: Clone,
    S: AliasSource,
    S::Target: QuerySource,
    <S::Target as QuerySource>::DefaultSelection: FieldAliasMapper<S>,
    <<S::Target as QuerySource>::DefaultSelection as FieldAliasMapper<S>>::Out:
        SelectableExpression<Self>,
{
    type FromClause = Self;
    type DefaultSelection =
        <<S::Target as QuerySource>::DefaultSelection as FieldAliasMapper<S>>::Out;

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.fields(self.source.target().default_selection())
    }
}

impl<S, DB> QueryFragment<DB> for Alias<S>
where
    S: AliasSource,
    DB: Backend,
    Self: QueryFragment<DB, DB::AliasSyntax>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::AliasSyntax>>::walk_ast(self, pass)
    }
}

impl<S, DB> QueryFragment<DB, sql_dialect::alias_syntax::AsAliasSyntax> for Alias<S>
where
    S: AliasSource,
    DB: Backend,
    S::Target: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.source.target().walk_ast(pass.reborrow())?;
        pass.push_sql(" AS ");
        pass.push_identifier(S::NAME)?;
        Ok(())
    }
}

impl<S> AsQuery for Alias<S>
where
    S: AliasSource,
    S::Target: AsQuery,
    Self: QuerySource,
    <Self as QuerySource>::DefaultSelection: ValidGrouping<()>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

/// This trait is used to allow external crates to implement
/// `AppearsInFromClause<QS> for Alias<S>`
///
/// at the table level without running in conflicting impl issues
///
/// Implementing this at the table level (in the crate that defines the tables)
/// does not result in conflicting impl issues because the table is the struct
/// we're implementing on.
pub trait AliasAppearsInFromClause<S, QS> {
    /// Will be passed on to the `impl AppearsInFromClause<QS>`
    type Count;
}
impl<S, QS> AppearsInFromClause<QS> for Alias<S>
where
    S: AliasSource,
    S::Target: AliasAppearsInFromClause<S, QS>,
{
    type Count = <S::Target as AliasAppearsInFromClause<S, QS>>::Count;
}

/// This trait is used to allow external crates to implement
/// `AppearsInFromClause<Alias<S2>> for Alias<S1>`
///
/// at the table level without running in conflicting impl issues
///
/// Implementing this at the table level (in the crate that defines the tables)
/// does not result in conflicting impl issues because the tables are specified as both first
/// argument of the trait and struct we're implementing on.
pub trait AliasAliasAppearsInFromClause<T2, S1, S2> {
    /// Will be passed on to the `impl AppearsInFromClause<QS>`
    type Count;
}
impl<T1, S1, S2> AliasAppearsInFromClause<S1, Alias<S2>> for T1
where
    S2: AliasSource,
    T1: AliasAliasAppearsInFromClause<S2::Target, S1, S2>,
{
    type Count = <T1 as AliasAliasAppearsInFromClause<S2::Target, S1, S2>>::Count;
}

/// This trait is used to allow external crates to implement
/// `AppearsInFromClause<Alias<S2>> for Alias<S1>`
///
/// at the alias level without running in conflicting impl issues
///
/// Implementing this at the alias level (in the crate that defines the aliases)
/// does not result in conflicting impl issues because the aliases are specified as both first
/// argument of the trait and struct we're implementing on.
///
/// The corresponding implementation of `AliasAliasAppearsInFromClause` for implementors of this
/// trait is done at the table level, to avoid conflict with the implementation for distinct tables
/// below.
pub trait AliasAliasAppearsInFromClauseSameTable<S2, T> {
    /// Will be passed on to the `impl AppearsInFromClause<QS>`
    type Count;
}

// impl<S: AliasSource<Table=T1>> AppearsInFromClause<T2> for Alias<S>
// where T1 != T2
impl<T1, T2, S> AliasAppearsInFromClause<S, T2> for T1
where
    T1: TableNotEqual<T2> + Table,
    T2: Table,
    S: AliasSource<Target = T1>,
{
    type Count = Never;
}

// impl<S1, S2> AppearsInFromClause<Alias<S1>> for Alias<S2>
// where S1: AliasSource, S2: AliasSource, S1::Table != S2::Table
impl<T1, T2, S1, S2> AliasAliasAppearsInFromClause<T1, S2, S1> for T2
where
    T1: TableNotEqual<T2> + Table,
    T2: Table,
    S1: AliasSource<Target = T1>,
    S2: AliasSource<Target = T2>,
{
    type Count = Never;
}

/// To be able to define `helper_types::Fields` with the usual conventions
///
/// This type is intentionally not publicly exported
#[allow(unreachable_pub)]
pub trait GetAliasSourceFromAlias {
    type Source;
}

impl<S> GetAliasSourceFromAlias for Alias<S> {
    type Source = S;
}
