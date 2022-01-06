use super::*;
use crate::associations::HasTable;
use crate::backend::Backend;
use crate::dsl::{Filter, Select};
use crate::expression::{
    self, is_aggregate, AppearsOnTable, Expression, SelectableExpression, ValidGrouping,
};
use crate::query_builder::nodes::StaticQueryFragment;
use crate::query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::query_dsl::methods::*;
use crate::query_dsl::InternalJoinDsl;
use crate::query_dsl::QueryDsl;
use crate::query_source::joins::ToInnerJoin;
use crate::query_source::joins::{
    AppendSelection, Inner, Join, JoinOn, LeftOuter, OnClauseWrapper,
};
use crate::query_source::{AppearsInFromClause, Column, Never, Once, QuerySource, Table};
use crate::result::QueryResult;

/// Types created by the `alias!` macro that serve to distinguish between aliases implement
/// this trait.
///
/// In order to be able to implement within diesel a lot of traits on what will represent the alias,
/// we cannot use directly that new type within the query builder. Instead, we will use `Alias<S>`,
/// where `S: AliasSource`.
///
/// This trait should never be implemented by an end-user directly.
pub trait AliasSource: Copy + Default {
    /// The name of this alias in the query
    const NAME: &'static str;
    /// The table it maps to
    type Table: Table;
}

#[derive(Debug, Clone, Copy, Default)]
/// Represents an alias within diesel's query builder
///
/// See [alias!] for more details.
pub struct Alias<S> {
    source: S,
}

#[derive(Debug, Clone, Copy, Default)]
/// Represents an aliased field (column) within diesel's query builder
///
/// See [alias!] for more details.
pub struct AliasedField<S, F> {
    _alias_source: S,
    _field: F,
}

impl<S> QueryId for Alias<S> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<S: AliasSource> Alias<S> {
    /// Maps a single field of the source table in this alias
    pub fn field<F>(self, field: F) -> AliasedField<S, F>
    where
        F: Column<Table = S::Table>,
    {
        AliasedField {
            _alias_source: self.source,
            _field: field,
        }
    }
    /// Maps multiple fields of the source table in this alias (takes in tuples)
    pub fn fields<Fields>(self, fields: Fields) -> <Fields as FieldAliasMapper<S>>::Out
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

impl<QS, S, C> AppearsOnTable<QS> for AliasedField<S, C>
where
    S: AliasSource,
    QS: AppearsInFromClause<Alias<S>, Count = Once>,
    C: Column<Table = S::Table>,
{
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

/// Serves to map `Self` to `Alias<S>`
///
/// Any column `Self` that belongs to `S::Table` will be transformed into `AliasedField<S, Self>`
///
/// Any column `Self` that does not belong to `S::Table` will be left untouched.
///
// This first part is implemented by the `table!` macro.
// The second part is useful to implement the joins, and may be useful to an end-user for
// ergonomics.
pub trait FieldAliasMapper<S> {
    /// Output type when mapping `C` to `Alias<S>`
    ///
    /// If `C: Column<Table = S::Table>`, `Out = AliasedField<S, C>`  
    /// Otherwise, `Out = C`
    type Out;

    /// Does the mapping
    fn map(self, alias: Alias<S>) -> Self::Out;
}

#[doc(hidden)]
/// Allows implementing `FieldAliasMapper` in external crates without running into conflicting impl
/// errors due to https://github.com/rust-lang/rust/issues/20400
///
/// We will always have `Self = S::Table` and `CT = C::Table`
pub trait FieldAliasMapperAssociatedTypesDisjointnessTrick<CT, S, C> {
    type Out;
    fn map(column: C, alias: Alias<S>) -> Self::Out;
}
impl<S, C> FieldAliasMapper<S> for C
where
    S: AliasSource,
    C: Column,
    S::Table: FieldAliasMapperAssociatedTypesDisjointnessTrick<C::Table, S, C>,
{
    type Out = <S::Table as FieldAliasMapperAssociatedTypesDisjointnessTrick<C::Table, S, C>>::Out;
    fn map(self, alias: Alias<S>) -> Self::Out {
        <S::Table as FieldAliasMapperAssociatedTypesDisjointnessTrick<C::Table, S, C>>::map(
            self, alias,
        )
    }
}

impl<TS, TC, S, C> FieldAliasMapperAssociatedTypesDisjointnessTrick<TC, S, C> for TS
where
    S: AliasSource<Table = TS>,
    C: Column<Table = TC>,
    TC: Table,
    TS: TableNotEqual<TC>,
{
    type Out = C;

    fn map(column: C, _alias: Alias<S>) -> Self::Out {
        // left untouched because the tables are different
        column
    }
}

macro_rules! field_alias_mapper {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<_S, $($T,)*> FieldAliasMapper<_S> for ($($T,)*)
            where
                _S: AliasSource,
                $($T: FieldAliasMapper<_S>,)*
            {
                type Out = ($(<$T as FieldAliasMapper<_S>>::Out,)*);

                fn map(self, alias: Alias<_S>) -> Self::Out {
                    (
                        $(self.$idx.map(alias),)*
                    )
                }
            }
        )*
    }
}

diesel_derives::__diesel_for_each_tuple!(field_alias_mapper);

// The following `FieldAliasMapper` impls are useful for the generic join implementations.
// More may be added.
impl<SPrev, SNew, F> FieldAliasMapper<SNew> for AliasedField<SPrev, F>
where
    SNew: AliasSource,
{
    type Out = Self;
    fn map(self, _alias: Alias<SNew>) -> Self::Out {
        // left untouched because it has already been aliased
        self
    }
}

impl<S, F> FieldAliasMapper<S> for expression::nullable::Nullable<F>
where
    F: FieldAliasMapper<S>,
{
    type Out = expression::nullable::Nullable<<F as FieldAliasMapper<S>>::Out>;
    fn map(self, alias: Alias<S>) -> Self::Out {
        expression::nullable::Nullable::new(self.0.map(alias))
    }
}

impl<S, F> FieldAliasMapper<S> for expression::grouped::Grouped<F>
where
    F: FieldAliasMapper<S>,
{
    type Out = expression::grouped::Grouped<<F as FieldAliasMapper<S>>::Out>;
    fn map(self, alias: Alias<S>) -> Self::Out {
        expression::grouped::Grouped(self.0.map(alias))
    }
}

impl<S> QuerySource for Alias<S>
where
    S: AliasSource,
    S::Table: QuerySource + HasTable<Table = S::Table>,
    <S::Table as QuerySource>::DefaultSelection: FieldAliasMapper<S>,
    <<S::Table as QuerySource>::DefaultSelection as FieldAliasMapper<S>>::Out:
        SelectableExpression<Self>,
{
    type FromClause = Self;
    type DefaultSelection =
        <<S::Table as QuerySource>::DefaultSelection as FieldAliasMapper<S>>::Out;

    fn from_clause(&self) -> Self::FromClause {
        *self
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

/// Declare a new alias for a table
///
/// Example usage
/// -------------
/// ```rust
/// # include!("../doctest_setup.rs");
/// fn main() {
///     use schema::users;
///     let connection = &mut establish_connection();
///     let (users1, users2) = diesel::alias!(schema::users as user1, schema::users as user2);
///     let res = users1
///         .inner_join(users2.on(users1.field(users::id).eq(users2.field(users::id))))
///         .select((users1.fields((users::id, users::name)), users2.field(users::name)))
///         .order_by(users2.field(users::id))
///         .load::<((i32, String), String)>(connection);
///     assert_eq!(
///         res,
///         Ok(vec![
///             ((1, "Sean".to_owned()), "Sean".to_owned()),
///             ((2, "Tess".to_owned()), "Tess".to_owned()),
///         ]),
///     );
/// }
/// ```
///
///
/// Make type expressable
/// ---------------------
/// It may sometimes be useful to declare an alias at the module level, in such a way that the type
/// of a query using it can be expressed (to not declare it anonymously).
///
/// This can be achieved in the following way
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel::{query_source::Alias, dsl};
///
/// diesel::alias!(schema::users as users_alias: UsersAlias);
///
/// fn some_function_that_returns_a_query_fragment(
/// ) -> dsl::InnerJoin<schema::posts::table, Alias<UsersAlias>>
/// {
///     schema::posts::table.inner_join(users_alias)
/// }
/// # fn main() {
/// #     some_function_that_returns_a_query_fragment();
/// # }
/// ```
///
/// Note that you may also use this form within a function, in the following way:
/// ```rust
/// # include!("../doctest_setup.rs");
/// fn main() {
///     diesel::alias!(schema::users as users_alias: UsersAlias);
///     users_alias.inner_join(schema::posts);
/// }
/// ```
///
/// Troubleshooting and limitations
/// -------------------------------
/// If you encounter a **compilation error** where "the trait
/// `AppearsInFromClause<Alias<your_alias>>` is not implemented", when trying to use two aliases to
/// the same table within a single query note the following two limitations:
///  - You will need to declare these in a single `alias!` call.
///  - The path to the table module will have to be expressed in the exact same
///    manner. (That is, you can do `alias!(schema::users as user1, schema::users as user2)`
///    or `alias!(users as user1, users as user2)`, but not
///    `alias!(schema::users as user1, users as user2)`)
#[macro_export]
macro_rules! alias {
    ($($($table: ident)::+ as $alias: ident),* $(,)?) => {{
        $crate::alias!(NoConst $($($table)::+ as $alias: $alias,)*);
        ($($crate::query_source::Alias::<$alias>::default()),*)
    }};
    ($($($table: ident)::+ as $alias_name: ident: $alias_ty: ident),* $(,)?) => {
        $crate::alias!(NoConst $($($table)::+ as $alias_name: $alias_ty,)*);
        $(
            #[allow(non_upper_case_globals)]
            const $alias_name: $crate::query_source::Alias::<$alias_ty> = $crate::query_source::Alias::new($alias_ty);
        )*
    };
    (NoConst $($($table: ident)::+ as $alias_name: ident: $alias_ty: ident),* $(,)?) => {
        $(
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default)]
            struct $alias_ty;

            impl $crate::query_source::AliasSource for $alias_ty {
                const NAME: &'static str = stringify!($alias_name);
                type Table = $($table)::+::table;
            }

            // impl AppearsInFromClause<Alias<$alias>> for Alias<$alias>
            impl $crate::query_source::AliasAliasAppearsInFromClause<$($table)::+::table, $alias_ty, $alias_ty> for $($table)::+::table {
                type Count = $crate::query_source::Once;
            }
        )*
        $crate::__internal_alias_helper!($($($table)::+ as $alias_ty,)*);
    };
}

#[macro_export]
#[doc(hidden)]
/// This only exists to hide internals from the doc
macro_rules! __internal_alias_helper {
    (
        $($left_table: ident)::+ as $left_alias: ident,
        $($right_table: ident)::+ as $right_alias: ident,
        $($($table: ident)::+ as $alias: ident,)*
    ) => {
        $crate::static_cond!{if ($($left_table)::+) == ($($right_table)::+) {
            impl $crate::query_source::AliasAliasAppearsInFromClause<$($left_table)::+::table, $right_alias, $left_alias>
                for $($right_table)::+::table
            {
                type Count = $crate::query_source::Never;
            }
            impl $crate::query_source::AliasAliasAppearsInFromClause<$($right_table)::+::table, $left_alias, $right_alias>
                for $($left_table)::+::table
            {
                type Count = $crate::query_source::Never;
            }
        }}
        $crate::__internal_alias_helper!($($right_table)::+ as $right_alias, $($($table)::+ as $alias,)*);
    };

    ($($table: ident)::+ as $alias: ident,) => {}
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

// FIXME: Remove this when overlapping marker traits are stable
impl<From, S, C> SelectableExpression<SelectStatement<FromClause<From>>> for AliasedField<S, C>
where
    Self: SelectableExpression<From> + AppearsOnTable<SelectStatement<FromClause<From>>>,
    From: QuerySource,
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

impl<S, Rhs, On> JoinTo<OnClauseWrapper<Rhs, On>> for Alias<S> {
    type FromClause = Rhs;
    type OnClause = On;

    fn join_target(rhs: OnClauseWrapper<Rhs, On>) -> (Self::FromClause, Self::OnClause) {
        (rhs.source, rhs.on)
    }
}

impl<T, S> JoinTo<T> for Alias<S>
where
    T: Table,
    S: AliasSource,
    S::Table: JoinTo<T>,
    <S::Table as JoinTo<T>>::OnClause: FieldAliasMapper<S>,
{
    type FromClause = <S::Table as JoinTo<T>>::FromClause;
    type OnClause = <<S::Table as JoinTo<T>>::OnClause as FieldAliasMapper<S>>::Out;

    fn join_target(rhs: T) -> (Self::FromClause, Self::OnClause) {
        let (from_clause, on_clause) = <S::Table as JoinTo<T>>::join_target(rhs);
        (from_clause, Self::default().fields(on_clause))
    }
}

impl<S2, S> JoinTo<Alias<S2>> for Alias<S>
where
    S2: AliasSource,
    S: AliasSource,
    S::Table: JoinTo<Alias<S2>>,
    <S::Table as JoinTo<Alias<S2>>>::OnClause: FieldAliasMapper<S>,
{
    type FromClause = <S::Table as JoinTo<Alias<S2>>>::FromClause;
    type OnClause = <<S::Table as JoinTo<Alias<S2>>>::OnClause as FieldAliasMapper<S>>::Out;

    fn join_target(rhs: Alias<S2>) -> (Self::FromClause, Self::OnClause) {
        let (from_clause, on_clause) = <S::Table as JoinTo<Alias<S2>>>::join_target(rhs);
        (from_clause, Self::default().fields(on_clause))
    }
}
