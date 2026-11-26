//! A set-returning function called as an item of the `FROM` clause

use crate::backend::{Backend, sql_dialect};
use crate::dsl;
use crate::expression::{
    AppearsOnTable, Expression, NonAggregate, SelectableExpression, TypedExpressionType,
    ValidGrouping,
};
use crate::query_builder::{
    AsQuery, AstPass, FromClause, NoFromClause, Query, QueryFragment, QueryId, SelectStatement,
    combination_clause,
};
use crate::query_dsl::methods::*;
use crate::query_dsl::{CombineDsl, InternalJoinDsl, QueryDsl, RunQueryDslSupport};
use crate::query_source::joins::{AppendSelection, OnClauseWrapper, ToInnerJoin};
use crate::query_source::{
    Alias, AliasSource, AliasedField, AppearsInFromClause, JoinTo, QueryRelationField, QuerySource,
    View,
};
use crate::result::QueryResult;

/// A function call in the `FROM` clause, named after the relation `R`
///
/// Returned by functions declared with `#[query_source(relation)]` in
/// [`declare_sql_function`](crate::declare_sql_function). `R` is the `view!`
/// listing the result columns, or an [`Alias`] of it after
/// [`aliased`](FunctionCall::aliased). `F` is the function call.
#[derive(Debug, Clone, Copy)]
pub struct FunctionCall<R, F> {
    relation: R,
    call: F,
}

impl<F> FunctionCall<F::Relation, F>
where
    F: TableFunction,
{
    #[doc(hidden)]
    /// Used by [`declare_sql_function`](crate::declare_sql_function)
    pub fn new(call: F) -> Self {
        Self {
            relation: Default::default(),
            call,
        }
    }
}

impl<V, F> FunctionCall<V, F>
where
    V: View,
{
    /// Names this call after an alias of its relation, for selecting
    /// from two calls of one function in a query
    ///
    /// The columns of an aliased call are read with
    /// [`Alias::field`].
    pub fn aliased<S>(self, alias: Alias<S>) -> FunctionCall<Alias<S>, F>
    where
        S: AliasSource<Target = V>,
    {
        FunctionCall {
            relation: alias,
            call: self.call,
        }
    }
}

/// Implemented by [`declare_sql_function`](crate::declare_sql_function) for
/// functions declared with `#[query_source(relation)]`
#[doc(hidden)]
pub trait TableFunction {
    /// The `view!` listing the result columns
    type Relation: View + Default + private::TableFunctionRelation;
    /// [`NoColumnAliases`] or, for `base_type`, [`BaseTypeColumnAlias`]
    type ColumnAliases: ColumnAliases<Self::Relation>;
}

/// The column alias list PostgreSQL appends to a call's alias
#[doc(hidden)]
pub trait ColumnAliases<R> {
    fn walk_ast<DB: Backend>(out: AstPass<'_, '_, DB>) -> QueryResult<()>;
}

/// Renders no column alias list
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct NoColumnAliases;

impl<R> ColumnAliases<R> for NoColumnAliases {
    fn walk_ast<DB: Backend>(_out: AstPass<'_, '_, DB>) -> QueryResult<()> {
        Ok(())
    }
}

/// Names the single column of a base type function after the relation's
/// column, because PostgreSQL names it after the alias
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct BaseTypeColumnAlias;

impl<R> ColumnAliases<R> for BaseTypeColumnAlias
where
    R: private::SingleColumnRelation,
{
    fn walk_ast<DB: Backend>(mut out: AstPass<'_, '_, DB>) -> QueryResult<()> {
        out.push_sql("(");
        out.push_identifier(<R::Column as QueryRelationField>::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

pub(crate) mod private {
    use crate::query_builder::nodes::{Identifier, StaticQueryFragment};
    use crate::query_source::{AliasSource, QueryRelation, QueryRelationField, View};

    /// A relation that can name a function call
    pub trait TableFunctionRelation {
        fn name() -> &'static str;
    }

    impl<V> TableFunctionRelation for V
    where
        V: View + StaticQueryFragment,
        V::Component: UnqualifiedName,
    {
        fn name() -> &'static str {
            V::STATIC_COMPONENT.name()
        }
    }

    impl<S> TableFunctionRelation for crate::query_source::Alias<S>
    where
        S: AliasSource,
        S::Target: View + StaticQueryFragment,
        <S::Target as StaticQueryFragment>::Component: UnqualifiedName,
    {
        fn name() -> &'static str {
            S::NAME
        }
    }

    /// The name of a relation declared without a schema
    #[diagnostic::on_unimplemented(
        message = "a function call in `FROM` must be named after a `view!` without a schema",
        note = "remove the schema from the `view!` declaration"
    )]
    pub trait UnqualifiedName {
        fn name(&'static self) -> &'static str;
    }

    impl UnqualifiedName for Identifier<'static> {
        fn name(&'static self) -> &'static str {
            self.0
        }
    }

    /// A relation with exactly one column
    pub trait SingleColumnRelation: QueryRelation {
        type Column: QueryRelationField;
    }

    impl<V> SingleColumnRelation for V
    where
        V: View + QueryRelation,
        V::AllColumns: SingleColumn,
    {
        type Column = <V::AllColumns as SingleColumn>::Column;
    }

    /// The columns of a relation that declares exactly one
    #[diagnostic::on_unimplemented(
        message = "the relation of a `base_type` function must declare exactly one column, found `{Self}`"
    )]
    pub trait SingleColumn {
        type Column: QueryRelationField;
    }

    impl<C> SingleColumn for (C,)
    where
        C: QueryRelationField,
    {
        type Column = C;
    }
}

impl<R, F> QueryId for FunctionCall<R, F>
where
    R: QueryId,
    F: QueryId,
{
    type QueryId = FunctionCall<R::QueryId, F::QueryId>;

    const HAS_STATIC_QUERY_ID: bool = R::HAS_STATIC_QUERY_ID && F::HAS_STATIC_QUERY_ID;
}

impl<R, F> QuerySource for FunctionCall<R, F>
where
    Self: Clone,
    R: QuerySource,
    R::DefaultSelection: SelectableExpression<Self>,
    F: AppearsOnTable<NoFromClause> + NonAggregate,
{
    type FromClause = Self;
    type DefaultSelection = R::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.relation.default_selection()
    }
}

impl<R, F> AsQuery for FunctionCall<R, F>
where
    Self: QuerySource,
    <Self as QuerySource>::DefaultSelection: ValidGrouping<()>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<R, F, QS> AppearsInFromClause<QS> for FunctionCall<R, F>
where
    R: AppearsInFromClause<QS>,
{
    type Count = R::Count;
}

impl<R, F, DB> QueryFragment<DB> for FunctionCall<R, F>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::TableFunctionSyntax>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::TableFunctionSyntax>>::walk_ast(self, pass)
    }
}

impl<R, F, DB> QueryFragment<DB, sql_dialect::table_function_syntax::PgLikeTableFunctions>
    for FunctionCall<R, F>
where
    DB: Backend,
    R: private::TableFunctionRelation,
    F: QueryFragment<DB> + TableFunction,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.call.walk_ast(out.reborrow())?;
        out.push_sql(" AS ");
        out.push_identifier(R::name())?;
        <F::ColumnAliases as ColumnAliases<F::Relation>>::walk_ast(out)
    }
}

impl<R, F, DB> QueryFragment<DB, sql_dialect::table_function_syntax::SqliteLikeTableFunctions>
    for FunctionCall<R, F>
where
    DB: Backend,
    R: private::TableFunctionRelation,
    F: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.call.walk_ast(out.reborrow())?;
        out.push_sql(" AS ");
        out.push_identifier(R::name())
    }
}

impl<S, C, F> SelectableExpression<FunctionCall<Alias<S>, F>> for AliasedField<S, C>
where
    S: AliasSource,
    C: QueryRelationField<QueryRelation = S::Target>,
    Self: AppearsOnTable<FunctionCall<Alias<S>, F>>,
{
}

impl<R, F> ToInnerJoin for FunctionCall<R, F> {
    type InnerJoin = Self;
}

impl<R, F, Selection> AppendSelection<Selection> for FunctionCall<R, F>
where
    Self: QuerySource,
{
    type Output = (<Self as QuerySource>::DefaultSelection, Selection);

    fn append_selection(&self, selection: Selection) -> Self::Output {
        (self.default_selection(), selection)
    }
}

impl<R, F, Rhs, On> JoinTo<OnClauseWrapper<Rhs, On>> for FunctionCall<R, F> {
    type FromClause = Rhs;
    type OnClause = On;

    fn join_target(rhs: OnClauseWrapper<Rhs, On>) -> (Self::FromClause, Self::OnClause) {
        (rhs.source, rhs.on)
    }
}

impl<R, F, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for FunctionCall<R, F>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <<Self as AsQuery>::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

impl<R, F> QueryDsl for FunctionCall<R, F> {}

impl<R, F> RunQueryDslSupport for FunctionCall<R, F> {}

impl<R, F, Predicate> FilterDsl<Predicate> for FunctionCall<R, F>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: FilterDsl<Predicate>,
{
    type Output = dsl::Filter<<Self as AsQuery>::Query, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}

impl<R, F, Predicate> OrFilterDsl<Predicate> for FunctionCall<R, F>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: OrFilterDsl<Predicate>,
{
    type Output = dsl::OrFilter<<Self as AsQuery>::Query, Predicate>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().or_filter(predicate)
    }
}

impl<R, F, Selection> SelectDsl<Selection> for FunctionCall<R, F>
where
    Selection: Expression,
    Self: AsQuery,
    <Self as AsQuery>::Query: SelectDsl<Selection>,
{
    type Output = <<Self as AsQuery>::Query as SelectDsl<Selection>>::Output;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}

impl<R, F> DistinctDsl for FunctionCall<R, F>
where
    Self: QuerySource + AsQuery<Query = SelectStatement<FromClause<Self>>>,
    <Self as QuerySource>::DefaultSelection:
        Expression<SqlType = <Self as AsQuery>::SqlType> + ValidGrouping<()>,
    <Self as AsQuery>::SqlType: TypedExpressionType,
{
    type Output = dsl::Distinct<SelectStatement<FromClause<Self>>>;

    fn distinct(self) -> dsl::Distinct<Self> {
        DistinctDsl::distinct(self.as_query())
    }
}

#[cfg(feature = "postgres_backend")]
impl<R, F, Selection> DistinctOnDsl<Selection> for FunctionCall<R, F>
where
    Selection: SelectableExpression<Self>,
    Self: QuerySource + AsQuery<Query = SelectStatement<FromClause<Self>>>,
    SelectStatement<FromClause<Self>>: DistinctOnDsl<Selection>,
    <Self as QuerySource>::DefaultSelection:
        Expression<SqlType = <Self as AsQuery>::SqlType> + ValidGrouping<()>,
    <Self as AsQuery>::SqlType: TypedExpressionType,
{
    type Output = dsl::DistinctOn<SelectStatement<FromClause<Self>>, Selection>;

    fn distinct_on(self, selection: Selection) -> dsl::DistinctOn<Self, Selection> {
        DistinctOnDsl::distinct_on(self.as_query(), selection)
    }
}

impl<R, F, Expr> GroupByDsl<Expr> for FunctionCall<R, F>
where
    Expr: Expression,
    Self: QuerySource + AsQuery<Query = SelectStatement<FromClause<Self>>>,
    <Self as QuerySource>::DefaultSelection:
        Expression<SqlType = <Self as AsQuery>::SqlType> + ValidGrouping<()>,
    <Self as AsQuery>::SqlType: TypedExpressionType,
    <Self as AsQuery>::Query: GroupByDsl<Expr>,
{
    type Output = dsl::GroupBy<SelectStatement<FromClause<Self>>, Expr>;

    fn group_by(self, expr: Expr) -> dsl::GroupBy<Self, Expr> {
        GroupByDsl::group_by(self.as_query(), expr)
    }
}

impl<R, F, Expr> OrderDsl<Expr> for FunctionCall<R, F>
where
    Expr: Expression,
    Self: AsQuery,
    <Self as AsQuery>::Query: OrderDsl<Expr>,
{
    type Output = <<Self as AsQuery>::Query as OrderDsl<Expr>>::Output;

    fn order(self, expr: Expr) -> Self::Output {
        self.as_query().order(expr)
    }
}

impl<R, F, Expr> ThenOrderDsl<Expr> for FunctionCall<R, F>
where
    Expr: Expression,
    Self: AsQuery,
    <Self as AsQuery>::Query: ThenOrderDsl<Expr>,
{
    type Output = <<Self as AsQuery>::Query as ThenOrderDsl<Expr>>::Output;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        self.as_query().then_order_by(expr)
    }
}

impl<R, F> LimitDsl for FunctionCall<R, F>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: LimitDsl,
{
    type Output = <<Self as AsQuery>::Query as LimitDsl>::Output;

    fn limit(self, limit: i64) -> Self::Output {
        self.as_query().limit(limit)
    }
}

impl<R, F> OffsetDsl for FunctionCall<R, F>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: OffsetDsl,
{
    type Output = <<Self as AsQuery>::Query as OffsetDsl>::Output;

    fn offset(self, offset: i64) -> Self::Output {
        self.as_query().offset(offset)
    }
}

impl<'a, R, F, DB> BoxedDsl<'a, DB> for FunctionCall<R, F>
where
    Self: QuerySource + AsQuery<Query = SelectStatement<FromClause<Self>>>,
    SelectStatement<FromClause<Self>>: BoxedDsl<'a, DB>,
    <Self as QuerySource>::DefaultSelection:
        Expression<SqlType = <Self as AsQuery>::SqlType> + ValidGrouping<()>,
    <Self as AsQuery>::SqlType: TypedExpressionType,
{
    type Output = dsl::IntoBoxed<'a, SelectStatement<FromClause<Self>>, DB>;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}

impl<'a, R, F, DB> BoxedCloneDsl<'a, DB> for FunctionCall<R, F>
where
    Self: QuerySource + AsQuery<Query = SelectStatement<FromClause<Self>>>,
    SelectStatement<FromClause<Self>>: BoxedCloneDsl<'a, DB>,
    <Self as QuerySource>::DefaultSelection:
        Expression<SqlType = <Self as AsQuery>::SqlType> + ValidGrouping<()>,
    <Self as AsQuery>::SqlType: TypedExpressionType,
{
    type Output = dsl::IntoBoxedClone<'a, SelectStatement<FromClause<Self>>, DB>;

    fn internal_into_boxed_clone(self) -> Self::Output {
        self.as_query().internal_into_boxed_clone()
    }
}

impl<R, F> CombineDsl for FunctionCall<R, F>
where
    Self: AsQuery,
{
    type Query = <Self as AsQuery>::Query;

    fn union<Rhs>(self, rhs: Rhs) -> dsl::Union<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <<Self as AsQuery>::Query as Query>::SqlType>,
    {
        combination_clause::CombinationClause::new(
            combination_clause::Union,
            combination_clause::Distinct,
            self.as_query(),
            rhs.as_query(),
        )
    }

    fn union_all<Rhs>(self, rhs: Rhs) -> dsl::UnionAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <<Self as AsQuery>::Query as Query>::SqlType>,
    {
        combination_clause::CombinationClause::new(
            combination_clause::Union,
            combination_clause::All,
            self.as_query(),
            rhs.as_query(),
        )
    }

    fn intersect<Rhs>(self, rhs: Rhs) -> dsl::Intersect<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <<Self as AsQuery>::Query as Query>::SqlType>,
    {
        combination_clause::CombinationClause::new(
            combination_clause::Intersect,
            combination_clause::Distinct,
            self.as_query(),
            rhs.as_query(),
        )
    }

    fn intersect_all<Rhs>(self, rhs: Rhs) -> dsl::IntersectAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <<Self as AsQuery>::Query as Query>::SqlType>,
    {
        combination_clause::CombinationClause::new(
            combination_clause::Intersect,
            combination_clause::All,
            self.as_query(),
            rhs.as_query(),
        )
    }

    fn except<Rhs>(self, rhs: Rhs) -> dsl::Except<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <<Self as AsQuery>::Query as Query>::SqlType>,
    {
        combination_clause::CombinationClause::new(
            combination_clause::Except,
            combination_clause::Distinct,
            self.as_query(),
            rhs.as_query(),
        )
    }

    fn except_all<Rhs>(self, rhs: Rhs) -> dsl::ExceptAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <<Self as AsQuery>::Query as Query>::SqlType>,
    {
        combination_clause::CombinationClause::new(
            combination_clause::Except,
            combination_clause::All,
            self.as_query(),
            rhs.as_query(),
        )
    }
}
