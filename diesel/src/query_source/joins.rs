use super::{AppearsInFromClause, Plus};
use crate::backend::Backend;
use crate::backend::DieselReserveSpecialization;
use crate::expression::grouped::Grouped;
use crate::expression::nullable::Nullable;
use crate::prelude::*;
use crate::query_builder::*;
use crate::query_dsl::InternalJoinDsl;
use crate::sql_types::BoolOrNullableBool;
use crate::util::TupleAppend;

/// A query source representing the join between two tables
pub struct Join<Left: QuerySource, Right: QuerySource, Kind> {
    left: FromClause<Left>,
    right: FromClause<Right>,
    kind: Kind,
}

impl<Left, Right, Kind> Clone for Join<Left, Right, Kind>
where
    Left: QuerySource,
    FromClause<Left>: Clone,
    Right: QuerySource,
    FromClause<Right>: Clone,
    Kind: Clone,
{
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            kind: self.kind.clone(),
        }
    }
}

impl<Left, Right, Kind> Copy for Join<Left, Right, Kind>
where
    Left: QuerySource,
    FromClause<Left>: Copy,
    Right: QuerySource,
    FromClause<Right>: Copy,
    Kind: Copy,
{
}

impl<Left, Right, Kind> std::fmt::Debug for Join<Left, Right, Kind>
where
    Left: QuerySource,
    FromClause<Left>: std::fmt::Debug,
    Right: QuerySource,
    FromClause<Right>: std::fmt::Debug,
    Kind: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Join")
            .field("left", &self.left)
            .field("right", &self.right)
            .field("kind", &self.kind)
            .finish()
    }
}

impl<Left, Right, Kind> QueryId for Join<Left, Right, Kind>
where
    Left: QueryId + QuerySource + 'static,
    Right: QueryId + QuerySource + 'static,
    Kind: QueryId,
{
    type QueryId = Join<Left, Right, Kind::QueryId>;

    const HAS_STATIC_QUERY_ID: bool =
        Left::HAS_STATIC_QUERY_ID && Right::HAS_STATIC_QUERY_ID && Kind::HAS_STATIC_QUERY_ID;
}

#[derive(Debug, Clone, Copy, QueryId)]
#[doc(hidden)]
/// A query source representing the join between two tables with an explicit
/// `ON` given. `Join` should usually be referenced instead, as all "type
/// safety" traits are implemented in terms of `Join` implementing them.
pub struct JoinOn<Join, On> {
    join: Join,
    on: On,
}

impl<Left, Right, Kind> Join<Left, Right, Kind>
where
    Left: QuerySource,
    Right: QuerySource,
{
    pub(crate) fn new(left: Left, right: Right, kind: Kind) -> Self {
        Join {
            left: FromClause::new(left),
            right: FromClause::new(right),
            kind,
        }
    }

    pub(crate) fn on<On>(self, on: On) -> JoinOn<Self, On> {
        JoinOn { join: self, on: on }
    }
}

impl<Left, Right> QuerySource for Join<Left, Right, Inner>
where
    Left: QuerySource + AppendSelection<Right::DefaultSelection>,
    Right: QuerySource,
    Left::Output: AppearsOnTable<Self>,
    Self: Clone,
{
    type FromClause = Self;
    // combining two valid selectable expressions for both tables will always yield a
    // valid selectable expressions for the whole join, so no need to check that here
    // again. These checked turned out to be quite expensive in terms of compile time
    // so we use a wrapper type to just skip the check and forward other more relevant
    // trait implementations to the inner type
    //
    // See https://github.com/diesel-rs/diesel/issues/3223 for details
    type DefaultSelection = self::private::SkipSelectableExpressionBoundCheckWrapper<Left::Output>;

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self::private::SkipSelectableExpressionBoundCheckWrapper(
            self.left
                .source
                .append_selection(self.right.source.default_selection()),
        )
    }
}

impl<Left, Right> QuerySource for Join<Left, Right, LeftOuter>
where
    Left: QuerySource + AppendSelection<Nullable<Right::DefaultSelection>>,
    Right: QuerySource,
    Left::Output: AppearsOnTable<Self>,
    Self: Clone,
{
    type FromClause = Self;
    // combining two valid selectable expressions for both tables will always yield a
    // valid selectable expressions for the whole join, so no need to check that here
    // again. These checked turned out to be quite expensive in terms of compile time
    // so we use a wrapper type to just skip the check and forward other more relevant
    // trait implementations to the inner type
    //
    // See https://github.com/diesel-rs/diesel/issues/3223 for details
    type DefaultSelection = self::private::SkipSelectableExpressionBoundCheckWrapper<Left::Output>;

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self::private::SkipSelectableExpressionBoundCheckWrapper(
            self.left
                .source
                .append_selection(self.right.source.default_selection().nullable()),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OnKeyword;

impl<DB: Backend> nodes::MiddleFragment<DB> for OnKeyword {
    fn push_sql(&self, mut pass: AstPass<'_, '_, DB>) {
        pass.push_sql(" ON ");
    }
}

impl<Join, On> QuerySource for JoinOn<Join, On>
where
    Join: QuerySource,
    On: AppearsOnTable<Join::FromClause> + Clone,
    On::SqlType: BoolOrNullableBool,
    Join::DefaultSelection: SelectableExpression<Self>,
{
    type FromClause = Grouped<nodes::InfixNode<Join::FromClause, On, OnKeyword>>;
    type DefaultSelection = Join::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        Grouped(nodes::InfixNode::new(
            self.join.from_clause(),
            self.on.clone(),
            OnKeyword,
        ))
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.join.default_selection()
    }
}

impl<Left, Right, Kind, DB> QueryFragment<DB> for Join<Left, Right, Kind>
where
    DB: Backend + DieselReserveSpecialization,
    Left: QuerySource,
    Left::FromClause: QueryFragment<DB>,
    Right: QuerySource,
    Right::FromClause: QueryFragment<DB>,
    Kind: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.left.from_clause.walk_ast(out.reborrow())?;
        self.kind.walk_ast(out.reborrow())?;
        out.push_sql(" JOIN ");
        self.right.from_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

/// Indicates that two tables can be joined without an explicit `ON` clause.
///
/// Implementations of this trait are generated by invoking [`joinable!`].
/// Implementing this trait means that you can call
/// `left_table.inner_join(right_table)`, without supplying the `ON` clause
/// explicitly. To join two tables which do not implement this trait, you will
/// need to call [`.on`].
///
/// See [`joinable!`] and [`inner_join`] for usage examples.
///
/// [`joinable!`]: crate::joinable!
/// [`.on`]: crate::query_dsl::JoinOnDsl::on()
/// [`inner_join`]: crate::query_dsl::QueryDsl::inner_join()
pub trait JoinTo<T> {
    #[doc(hidden)]
    type FromClause;
    #[doc(hidden)]
    type OnClause;
    #[doc(hidden)]
    fn join_target(rhs: T) -> (Self::FromClause, Self::OnClause);
}

#[doc(hidden)]
/// Used to ensure the sql type of `left.join(mid).join(right)` is
/// `(Left, Mid, Right)` and not `((Left, Mid), Right)`. This needs
/// to be separate from `TupleAppend` because we still want to keep
/// the column lists (which are tuples) separate.
pub trait AppendSelection<Selection> {
    type Output;

    fn append_selection(&self, selection: Selection) -> Self::Output;
}

impl<T: Table, Selection> AppendSelection<Selection> for T {
    type Output = (T::AllColumns, Selection);

    fn append_selection(&self, selection: Selection) -> Self::Output {
        (T::all_columns(), selection)
    }
}

impl<Left, Mid, Selection, Kind> AppendSelection<Selection> for Join<Left, Mid, Kind>
where
    Left: QuerySource,
    Mid: QuerySource,
    Self: QuerySource,
    <Self as QuerySource>::DefaultSelection: TupleAppend<Selection>,
{
    type Output = <<Self as QuerySource>::DefaultSelection as TupleAppend<Selection>>::Output;

    fn append_selection(&self, selection: Selection) -> Self::Output {
        self.default_selection().tuple_append(selection)
    }
}

impl<Join, On, Selection> AppendSelection<Selection> for JoinOn<Join, On>
where
    Join: AppendSelection<Selection>,
{
    type Output = Join::Output;

    fn append_selection(&self, selection: Selection) -> Self::Output {
        self.join.append_selection(selection)
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, QueryId)]
pub struct Inner;

impl<DB> QueryFragment<DB> for Inner
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" INNER");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, QueryId)]
pub struct LeftOuter;

impl<DB> QueryFragment<DB> for LeftOuter
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" LEFT OUTER");
        Ok(())
    }
}

impl<Left, Mid, Right, Kind> JoinTo<Right> for Join<Left, Mid, Kind>
where
    Left: JoinTo<Right> + QuerySource,
    Mid: QuerySource,
{
    type FromClause = <Left as JoinTo<Right>>::FromClause;
    type OnClause = Left::OnClause;

    fn join_target(rhs: Right) -> (Self::FromClause, Self::OnClause) {
        Left::join_target(rhs)
    }
}

impl<Join, On, Right> JoinTo<Right> for JoinOn<Join, On>
where
    Join: JoinTo<Right>,
{
    type FromClause = Join::FromClause;
    type OnClause = Join::OnClause;

    fn join_target(rhs: Right) -> (Self::FromClause, Self::OnClause) {
        Join::join_target(rhs)
    }
}

impl<T, Left, Right, Kind> AppearsInFromClause<T> for Join<Left, Right, Kind>
where
    Left: AppearsInFromClause<T> + QuerySource,
    Right: AppearsInFromClause<T> + QuerySource,
    Left::Count: Plus<Right::Count>,
{
    type Count = <Left::Count as Plus<Right::Count>>::Output;
}

impl<T, Join, On> AppearsInFromClause<T> for JoinOn<Join, On>
where
    Join: AppearsInFromClause<T>,
{
    type Count = Join::Count;
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnClauseWrapper<Source, On> {
    pub(crate) source: Source,
    pub(crate) on: On,
}

impl<Source, On> OnClauseWrapper<Source, On> {
    pub fn new(source: Source, on: On) -> Self {
        OnClauseWrapper { source, on }
    }
}

impl<Lhs, Rhs, On> JoinTo<OnClauseWrapper<Rhs, On>> for Lhs
where
    Lhs: Table,
{
    type FromClause = Rhs;
    type OnClause = On;

    fn join_target(rhs: OnClauseWrapper<Rhs, On>) -> (Self::FromClause, Self::OnClause) {
        (rhs.source, rhs.on)
    }
}

impl<Lhs, Rhs, On> JoinTo<Rhs> for OnClauseWrapper<Lhs, On>
where
    Lhs: JoinTo<Rhs>,
{
    type FromClause = <Lhs as JoinTo<Rhs>>::FromClause;
    type OnClause = <Lhs as JoinTo<Rhs>>::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        <Lhs as JoinTo<Rhs>>::join_target(rhs)
    }
}

impl<Rhs, Kind, On1, On2, Lhs> InternalJoinDsl<Rhs, Kind, On1> for OnClauseWrapper<Lhs, On2>
where
    Lhs: InternalJoinDsl<Rhs, Kind, On1>,
{
    type Output = OnClauseWrapper<<Lhs as InternalJoinDsl<Rhs, Kind, On1>>::Output, On2>;

    fn join(self, rhs: Rhs, kind: Kind, on: On1) -> Self::Output {
        OnClauseWrapper {
            source: self.source.join(rhs, kind, on),
            on: self.on,
        }
    }
}

impl<Qs, On> QueryDsl for OnClauseWrapper<Qs, On> {}

#[doc(hidden)]
/// Convert any joins in a `FROM` clause into an inner join.
///
/// This trait is used to determine whether
/// `Nullable<T>: SelectableExpression<SomeJoin>`. We consider it to be
/// selectable if `T: SelectableExpression<InnerJoin>`. Since `SomeJoin`
/// may be deeply nested, we need to recursively change any appearances of
/// `LeftOuter` to `Inner` in order to perform this check.
pub trait ToInnerJoin {
    type InnerJoin;
}

impl<Left, Right, Kind> ToInnerJoin for Join<Left, Right, Kind>
where
    Left: ToInnerJoin + QuerySource,
    Left::InnerJoin: QuerySource,
    Right: ToInnerJoin + QuerySource,
    Right::InnerJoin: QuerySource,
{
    type InnerJoin = Join<Left::InnerJoin, Right::InnerJoin, Inner>;
}

impl<Join, On> ToInnerJoin for JoinOn<Join, On>
where
    Join: ToInnerJoin,
{
    type InnerJoin = JoinOn<Join::InnerJoin, On>;
}

impl<From> ToInnerJoin for SelectStatement<FromClause<From>>
where
    From: ToInnerJoin + QuerySource,
    From::InnerJoin: QuerySource,
{
    type InnerJoin = SelectStatement<FromClause<From::InnerJoin>>;
}

impl<T: Table> ToInnerJoin for T {
    type InnerJoin = T;
}

mod private {
    use crate::backend::Backend;
    use crate::expression::{Expression, ValidGrouping};
    use crate::query_builder::{AstPass, QueryFragment, SelectClauseExpression};
    use crate::{AppearsOnTable, QueryResult, SelectableExpression};

    #[derive(Debug, crate::query_builder::QueryId, Copy, Clone)]
    pub struct SkipSelectableExpressionBoundCheckWrapper<T>(pub(super) T);

    impl<DB, T> QueryFragment<DB> for SkipSelectableExpressionBoundCheckWrapper<T>
    where
        T: QueryFragment<DB>,
        DB: Backend,
    {
        fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
            self.0.walk_ast(pass)
        }
    }

    // The default select clause is only valid for no group by clause
    // anyway so we can just skip the recursive check here
    impl<T> ValidGrouping<()> for SkipSelectableExpressionBoundCheckWrapper<T> {
        type IsAggregate = crate::expression::is_aggregate::No;
    }

    // This needs to use the expression impl
    impl<QS, T> SelectClauseExpression<QS> for SkipSelectableExpressionBoundCheckWrapper<T>
    where
        T: SelectClauseExpression<QS>,
    {
        type Selection = T::Selection;

        type SelectClauseSqlType = T::SelectClauseSqlType;
    }

    // The default select clause for joins is always valid assuming that
    // the default select clause of all involved query sources is
    // valid too. We can skip the recursive check here.
    // This is the main optimization.
    impl<QS, T> SelectableExpression<QS> for SkipSelectableExpressionBoundCheckWrapper<T> where
        Self: AppearsOnTable<QS>
    {
    }

    impl<QS, T> AppearsOnTable<QS> for SkipSelectableExpressionBoundCheckWrapper<T> where
        Self: Expression
    {
    }

    // Expression must recurse the whole expression
    // as this is required for the return type of the query
    impl<T> Expression for SkipSelectableExpressionBoundCheckWrapper<T>
    where
        T: Expression,
    {
        type SqlType = T::SqlType;
    }

    impl<T, Selection> crate::util::TupleAppend<Selection>
        for SkipSelectableExpressionBoundCheckWrapper<T>
    where
        T: crate::util::TupleAppend<Selection>,
    {
        // We're re-wrapping after anyway
        type Output = T::Output;

        fn tuple_append(self, right: Selection) -> Self::Output {
            self.0.tuple_append(right)
        }
    }
}
