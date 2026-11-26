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
use core::marker::PhantomData;

/// A function call in the `FROM` clause, named after the relation `R`
///
/// Returned by functions declared with `#[query_source(relation)]` in
/// [`declare_sql_function`](crate::declare_sql_function). `R` is the `view!`
/// listing the result columns, or an [`Alias`] of it after
/// [`aliased`](FunctionCall::aliased). `F` is the function call.
#[derive(Debug, Clone, Copy)]
pub struct FunctionCall<R, F, L = NoFromClause> {
    relation: R,
    call: F,
    lateral: PhantomData<L>,
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
            lateral: PhantomData,
        }
    }
}

impl<V, F, L> FunctionCall<V, F, L>
where
    V: View,
{
    /// Names this call after an alias of its relation, for selecting
    /// from two calls of one function in a query
    ///
    /// The columns of an aliased call are read with
    /// [`Alias::field`].
    pub fn aliased<S>(self, alias: Alias<S>) -> FunctionCall<Alias<S>, F, L>
    where
        S: AliasSource<Target = V>,
    {
        FunctionCall {
            relation: alias,
            call: self.call,
            lateral: PhantomData,
        }
    }
}

impl<R, F> FunctionCall<R, F> {
    /// Supplies the join predicate for this function call.
    pub fn on<On>(self, predicate: On) -> OnClauseWrapper<Self, On> {
        OnClauseWrapper::new(self, predicate)
    }

    /// Builds a subselect whose arguments may reference the enclosing query.
    ///
    /// # Example
    ///
    /// ```
    /// # // Reading the query result requires reading memory allocated by the
    /// # // native sqlite library, which miri cannot track provenance for.
    /// # #[cfg(all(any(feature = "sqlite", feature = "postgres"), not(miri)))]
    /// # {
    /// use diesel::dsl::exists;
    /// use diesel::prelude::*;
    /// use diesel::sql_types::{Integer, Text};
    ///
    /// diesel::view! {
    ///     json_each {
    ///         key -> Integer,
    ///         value -> Text,
    ///     }
    /// }
    /// # #[cfg(not(feature = "postgres"))]
    /// #[diesel::declare_sql_function]
    /// extern "SQL" {
    ///     #[query_source(json_each)]
    ///     fn json_each(json: Text) -> json_each::SqlType;
    /// }
    ///
    /// # #[cfg(feature = "postgres")]
    /// # #[diesel::declare_sql_function]
    /// # extern "SQL" {
    /// #     #[query_source(json_each)]
    /// #     #[sql_name = "pg_temp.json_each"]
    /// #     fn json_each(json: Text) -> json_each::SqlType;
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # let connection = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # #[cfg(feature = "postgres")]
    /// # let connection = &mut {
    /// #     dotenvy::dotenv().ok();
    /// #     let database_url = std::env::var("PG_DATABASE_URL")
    /// #         .or_else(|_| std::env::var("DATABASE_URL")).unwrap();
    /// #     PgConnection::establish(&database_url).unwrap()
    /// # };
    /// # #[cfg(feature = "postgres")]
    /// # {
    /// #     use diesel::connection::SimpleConnection;
    /// #     connection.batch_execute(
    /// #         "CREATE FUNCTION pg_temp.json_each(input text) RETURNS TABLE(key integer, value text)
    /// #          LANGUAGE SQL AS 'SELECT (ordinality - 1)::integer, value
    /// #          FROM json_array_elements_text(input::json) WITH ORDINALITY AS elements(value, ordinality)'"
    /// #     )?;
    /// # }
    /// let (outer, inner) = diesel::alias!(json_each as outer, json_each as inner);
    /// let subselect = json_each(outer.field(json_each::value))
    ///     .aliased(inner)
    ///     .correlated()
    ///     .select(inner.field(json_each::value));
    /// let rows = json_each(r#"["[\"a\"]","[]"]"#)
    ///     .aliased(outer)
    ///     .filter(exists(subselect))
    ///     .select(outer.field(json_each::key))
    ///     .load::<i32>(connection)?;
    /// assert_eq!(rows, [0]);
    /// # }
    /// # Ok::<(), diesel::result::Error>(())
    /// ```
    pub fn correlated(self) -> dsl::Correlated<Self>
    where
        FunctionCall<R, F, private::Outer>: QuerySource,
    {
        crate::query_source::CorrelatedSelect::new(SelectStatement::simple(FunctionCall {
            relation: self.relation,
            call: self.call,
            lateral: PhantomData,
        }))
    }

    pub(crate) fn lateral(self) -> FunctionCall<R, F, private::Lateral> {
        FunctionCall {
            relation: self.relation,
            call: self.call,
            lateral: PhantomData,
        }
    }
}

#[doc(hidden)]
pub trait CorrelatedDsl {
    type Output;
}

impl<Relation, Call> CorrelatedDsl for FunctionCall<Relation, Call>
where
    FunctionCall<Relation, Call, private::Outer>: QuerySource,
{
    type Output = crate::query_source::CorrelatedSelect<
        SelectStatement<FromClause<FunctionCall<Relation, Call, private::Outer>>>,
        Self,
    >;
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

    #[derive(Debug, Clone, Copy, crate::query_builder::QueryId)]
    pub struct Lateral;

    #[derive(Debug, Clone, Copy, crate::query_builder::QueryId)]
    pub struct Outer;

    #[diagnostic::on_unimplemented(
        message = "function arguments cannot reference columns in an ordinary join",
        note = "use `inner_join_lateral` or `left_join_lateral` for column arguments"
    )]
    pub trait ValidArguments<F> {}

    #[diagnostic::do_not_recommend]
    impl<F: crate::expression::AppearsOnTable<crate::query_builder::NoFromClause>> ValidArguments<F>
        for crate::query_builder::NoFromClause
    {
    }

    impl<F> ValidArguments<F> for Lateral {}
    impl<F> ValidArguments<F> for Outer {}

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

impl<R, F, L> QueryId for FunctionCall<R, F, L>
where
    R: QueryId,
    F: QueryId,
    L: QueryId + 'static,
{
    type QueryId = FunctionCall<R::QueryId, F::QueryId, L>;

    const HAS_STATIC_QUERY_ID: bool = R::HAS_STATIC_QUERY_ID && F::HAS_STATIC_QUERY_ID;
}

impl<R, F, L> QuerySource for FunctionCall<R, F, L>
where
    Self: Clone,
    R: QuerySource,
    R::DefaultSelection: SelectableExpression<Self>,
    F: NonAggregate,
    L: private::ValidArguments<F>,
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

impl<R, F, L> AsQuery for FunctionCall<R, F, L>
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

impl<R, F, L, QS> AppearsInFromClause<QS> for FunctionCall<R, F, L>
where
    R: AppearsInFromClause<QS>,
{
    type Count = R::Count;
}

impl<R, F, L, DB> QueryFragment<DB> for FunctionCall<R, F, L>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::TableFunctionSyntax>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::TableFunctionSyntax>>::walk_ast(self, pass)
    }
}

impl<R, F, L, DB> QueryFragment<DB, sql_dialect::table_function_syntax::PgLikeTableFunctions>
    for FunctionCall<R, F, L>
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

impl<R, F, L, DB> QueryFragment<DB, sql_dialect::table_function_syntax::SqliteLikeTableFunctions>
    for FunctionCall<R, F, L>
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

impl<S, C, F, L> SelectableExpression<FunctionCall<Alias<S>, F, L>> for AliasedField<S, C>
where
    S: AliasSource,
    C: QueryRelationField<QueryRelation = S::Target>,
    Self: AppearsOnTable<FunctionCall<Alias<S>, F, L>>,
{
}

impl<R, F, L> ToInnerJoin for FunctionCall<R, F, L> {
    type InnerJoin = Self;
}

impl<R, F, L, Selection> AppendSelection<Selection> for FunctionCall<R, F, L>
where
    Self: QuerySource,
{
    type Output = (<Self as QuerySource>::DefaultSelection, Selection);

    fn append_selection(&self, selection: Selection) -> Self::Output {
        (self.default_selection(), selection)
    }
}

impl<R, F, L, Rhs, On> JoinTo<OnClauseWrapper<Rhs, On>> for FunctionCall<R, F, L> {
    type FromClause = Rhs;
    type OnClause = On;

    fn join_target(rhs: OnClauseWrapper<Rhs, On>) -> (Self::FromClause, Self::OnClause) {
        (rhs.source, rhs.on)
    }
}

impl<R, F, L, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for FunctionCall<R, F, L>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <<Self as AsQuery>::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

impl<R, F, L> QueryDsl for FunctionCall<R, F, L> {}

impl<R, F, L> RunQueryDslSupport for FunctionCall<R, F, L> {}

impl<R, F, L, Predicate> FilterDsl<Predicate> for FunctionCall<R, F, L>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: FilterDsl<Predicate>,
{
    type Output = dsl::Filter<<Self as AsQuery>::Query, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}

impl<R, F, L, Predicate> OrFilterDsl<Predicate> for FunctionCall<R, F, L>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: OrFilterDsl<Predicate>,
{
    type Output = dsl::OrFilter<<Self as AsQuery>::Query, Predicate>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().or_filter(predicate)
    }
}

impl<R, F, L, Selection> SelectDsl<Selection> for FunctionCall<R, F, L>
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

impl<R, F, L> DistinctDsl for FunctionCall<R, F, L>
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
impl<R, F, L, Selection> DistinctOnDsl<Selection> for FunctionCall<R, F, L>
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

impl<R, F, L, Expr> GroupByDsl<Expr> for FunctionCall<R, F, L>
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

impl<R, F, L, Expr> OrderDsl<Expr> for FunctionCall<R, F, L>
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

impl<R, F, L, Expr> ThenOrderDsl<Expr> for FunctionCall<R, F, L>
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

impl<R, F, L> LimitDsl for FunctionCall<R, F, L>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: LimitDsl,
{
    type Output = <<Self as AsQuery>::Query as LimitDsl>::Output;

    fn limit(self, limit: i64) -> Self::Output {
        self.as_query().limit(limit)
    }
}

impl<R, F, L> OffsetDsl for FunctionCall<R, F, L>
where
    Self: AsQuery,
    <Self as AsQuery>::Query: OffsetDsl,
{
    type Output = <<Self as AsQuery>::Query as OffsetDsl>::Output;

    fn offset(self, offset: i64) -> Self::Output {
        self.as_query().offset(offset)
    }
}

impl<'a, R, F, L, DB> BoxedDsl<'a, DB> for FunctionCall<R, F, L>
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

impl<'a, R, F, L, DB> BoxedCloneDsl<'a, DB> for FunctionCall<R, F, L>
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

impl<R, F, L> CombineDsl for FunctionCall<R, F, L>
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
