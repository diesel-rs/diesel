use prelude::*;
use expression::SelectableExpression;
use expression::nullable::Nullable;
use query_builder::*;
use result::QueryResult;
use super::{QuerySource, Table};

#[derive(Debug, Clone, Copy)]
/// A query source representing the join between two tables
pub struct Join<Left, Right, Kind> {
    left: Left,
    right: Right,
    kind: Kind,
}

impl<Left, Right, Kind> Join<Left, Right, Kind> {
    pub fn new(left: Left, right: Right) -> Self where
        Kind: Default,
    {
        Join {
            left: left,
            right: right,
            kind: Kind::default(),
        }
    }
}

impl<Left, Right, Kind> AsQuery for Join<Left, Right, Kind> where
    SelectStatement<Join<Left, Right, Kind>>: Query,
{
    type SqlType = <SelectStatement<Self> as Query>::SqlType;
    type Query = SelectStatement<Self>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl_query_id!(Join<Left, Right, Kind>);

impl<Left, Right> QuerySource for Join<Left, Right, Inner> where
    Left: Table + JoinTo<Right, Inner>,
    Right: Table,
    (Left::AllColumns, Right::AllColumns): SelectableExpression<
        Join<Left, Right, Inner>,
    >,
{
    type FromClause = <Left as JoinTo<Right, Inner>>::JoinClause;
    type DefaultSelection = (Left::AllColumns, Right::AllColumns);

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(self.kind)
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        (Left::all_columns(), Right::all_columns())
    }
}

impl<Left, Right> QuerySource for Join<Left, Right, LeftOuter> where
    Left: Table + JoinTo<Right, LeftOuter>,
    Right: Table,
    (Left::AllColumns, Nullable<Right::AllColumns>): SelectableExpression<
        Join<Left, Right, LeftOuter>,
    >,
{
    type FromClause = <Left as JoinTo<Right, LeftOuter>>::JoinClause;
    type DefaultSelection = (Left::AllColumns, Nullable<Right::AllColumns>);

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(self.kind)
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        (Left::all_columns(), Right::all_columns().nullable())
    }
}

impl<Left, Right, T> SelectableExpression<Join<Left, Right, LeftOuter>>
    for Nullable<T> where
        T: SelectableExpression<Join<Left, Right, Inner>>,
        Nullable<T>: AppearsOnTable<Join<Left, Right, LeftOuter>>,
{
}

/// Indicates that two tables can be used together in a JOIN clause.
/// Implementations of this trait will be generated for you automatically by
/// the [association annotations](FIXME: Add link) from codegen.
pub trait JoinTo<T: Table, JoinType>: Table {
    #[doc(hidden)]
    type JoinClause;
    #[doc(hidden)]
    fn join_clause(&self, join_type: JoinType) -> Self::JoinClause;
}

use backend::Backend;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Inner;
impl_query_id!(Inner);

impl<DB: Backend> QueryFragment<DB> for Inner {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(" INNER");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LeftOuter;
impl_query_id!(LeftOuter);

impl<DB: Backend> QueryFragment<DB> for LeftOuter {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(" LEFT OUTER");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}
