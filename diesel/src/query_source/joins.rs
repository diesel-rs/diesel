use prelude::*;
use expression::SelectableExpression;
use expression::nullable::Nullable;
use query_builder::*;
use result::QueryResult;
use super::QuerySource;

#[derive(Debug, Clone, Copy)]
/// A query source representing the join between two tables
pub struct Join<Left, Right, Kind> {
    left: Left,
    right: Right,
    kind: Kind,
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
/// A query source representing the join between two tables with an explicit
/// `ON` given. `Join` should usually be referenced instead, as all "type
/// safety" traits are implemented in terms of `Join` implementing them.
pub struct JoinOn<Join, On> {
    join: Join,
    on: On,
}

impl<Left, Right, Kind> Join<Left, Right, Kind> {
    pub fn new(left: Left, right: Right, kind: Kind) -> Self {
        Join {
            left: left,
            right: right,
            kind: kind,
        }
    }

    #[doc(hidden)]
    pub fn on<On>(self, on: On) -> JoinOn<Self, On> {
        JoinOn {
            join: self,
            on: on,
        }
    }
}

impl_query_id!(Join<Left, Right, Kind>);
impl_query_id!(JoinOn<Join, On>);

impl<Left, Right> QuerySource for Join<Left, Right, Inner> where
    Left: QuerySource + JoinTo<Right>,
    Right: QuerySource,
    (Left::DefaultSelection, Right::DefaultSelection): SelectableExpression<Self>,
    Self: Clone,
{
    type FromClause = Self;
    type DefaultSelection = (Left::DefaultSelection, Right::DefaultSelection);

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        (self.left.default_selection(), self.right.default_selection())
    }
}

impl<Left, Right> QuerySource for Join<Left, Right, LeftOuter> where
    Left: QuerySource + JoinTo<Right>,
    Right: QuerySource,
    (Left::DefaultSelection, Nullable<Right::DefaultSelection>): SelectableExpression<Self>,
    Self: Clone,
{
    type FromClause = Self;
    type DefaultSelection = (Left::DefaultSelection, Nullable<Right::DefaultSelection>);

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        (self.left.default_selection(), self.right.default_selection().nullable())
    }
}

impl<Join, On> QuerySource for JoinOn<Join, On> where
    Join: QuerySource,
    On: AppearsOnTable<Join::FromClause> + Clone,
    Join::DefaultSelection: SelectableExpression<Self>,
{
    type FromClause = nodes::InfixNode<'static, Join::FromClause, On>;
    type DefaultSelection = Join::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        nodes::InfixNode::new(
            self.join.from_clause(),
            self.on.clone(),
            " ON ",
        )
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.join.default_selection()
    }
}

impl<Left, Right, Kind, DB> QueryFragment<DB> for Join<Left, Right, Kind> where
    DB: Backend,
    Left: QuerySource,
    Left::FromClause: QueryFragment<DB>,
    Right: QuerySource,
    Right::FromClause: QueryFragment<DB>,
    Kind: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.left.from_clause().walk_ast(out.reborrow())?;
        self.kind.walk_ast(out.reborrow())?;
        out.push_sql(" JOIN ");
        self.right.from_clause().walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Left, Right, T> SelectableExpression<Join<Left, Right, LeftOuter>>
    for Nullable<T> where
        T: SelectableExpression<Join<Left, Right, Inner>>,
        Nullable<T>: AppearsOnTable<Join<Left, Right, LeftOuter>>,
{
}

// FIXME: Remove this when overlapping marker traits are stable
impl<Left, Right, On, T> SelectableExpression<JoinOn<Join<Left, Right, LeftOuter>, On>>
    for Nullable<T> where
        T: SelectableExpression<Join<Left, Right, Inner>>,
        Nullable<T>: AppearsOnTable<JoinOn<Join<Left, Right, LeftOuter>, On>>,
{
}

// FIXME: We want these blanket impls when overlapping marker traits are stable
// impl<T, Join, On> SelectableExpression<JoinOn<Join, On>> for T where
//     T: SelectableExpression<Join> + AppearsOnTable<JoinOn<Join, On>>,
// {
// }

/// Indicates that two tables can be used together in a JOIN clause.
/// Implementations of this trait will be generated for you automatically by
/// the [association annotations](../associations/index.html) from codegen.
pub trait JoinTo<T> {
    #[doc(hidden)]
    type JoinOnClause;
    #[doc(hidden)]
    fn join_on_clause() -> Self::JoinOnClause;
}

use backend::Backend;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Inner;
impl_query_id!(Inner);

impl<DB: Backend> QueryFragment<DB> for Inner {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" INNER");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LeftOuter;
impl_query_id!(LeftOuter);

impl<DB: Backend> QueryFragment<DB> for LeftOuter {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" LEFT OUTER");
        Ok(())
    }
}

use super::{Succ, Never, ContainsTable};

impl<T, Left, Right, Kind> ContainsTable<T> for Join<Left, Right, Kind> where
    Left: ContainsTable<T>,
    Right: ContainsTable<T>,
    Left::Count: Plus<Right::Count>,
{
    type Count = <Left::Count as Plus<Right::Count>>::Output;
}

impl<T, Join, On> ContainsTable<T> for JoinOn<Join, On> where
    Join: ContainsTable<T>,
{
    type Count = Join::Count;
}

pub trait Plus<T> {
    type Output;
}

impl<T, U> Plus<T> for Succ<U> where
    U: Plus<T>,
{
    type Output = Succ<U::Output>;
}

impl<T> Plus<T> for Never {
    type Output = T;
}
