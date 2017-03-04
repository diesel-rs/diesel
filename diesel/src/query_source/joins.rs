use prelude::*;
use expression::SelectableExpression;
use expression::nullable::Nullable;
use query_builder::*;
use result::QueryResult;
use super::{QuerySource, Table};

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct InnerJoinSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> InnerJoinSource<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        InnerJoinSource {
            left: left,
            right: right,
        }
    }
}

impl<Left, Right> QuerySource for InnerJoinSource<Left, Right> where
    Left: Table + JoinTo<Right, Inner>,
    Right: Table,
    (Left::AllColumns, Right::AllColumns): SelectableExpression<
        InnerJoinSource<Left, Right>,
    >,
{
    type FromClause = <Left as JoinTo<Right, Inner>>::JoinClause;
    type DefaultSelection = (Left::AllColumns, Right::AllColumns);

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(Inner)
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        (Left::all_columns(), Right::all_columns())
    }
}

impl<Left, Right> AsQuery for InnerJoinSource<Left, Right> where
    SelectStatement<InnerJoinSource<Left, Right>>: Query,
{
    type SqlType = <SelectStatement<Self> as Query>::SqlType;
    type Query = SelectStatement<Self>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl_query_id!(InnerJoinSource<Left, Right>);

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct LeftOuterJoinSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> LeftOuterJoinSource<Left, Right> {
    pub fn new(left: Left, right: Right) -> Self {
        LeftOuterJoinSource {
            left: left,
            right: right,
        }
    }
}

impl<Left, Right> QuerySource for LeftOuterJoinSource<Left, Right> where
    Left: Table + JoinTo<Right, LeftOuter>,
    Right: Table,
    (Left::AllColumns, Nullable<Right::AllColumns>): SelectableExpression<
        LeftOuterJoinSource<Left, Right>,
    >,
{
    type FromClause = <Left as JoinTo<Right, LeftOuter>>::JoinClause;
    type DefaultSelection = (Left::AllColumns, Nullable<Right::AllColumns>);

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(LeftOuter)
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        (Left::all_columns(), Right::all_columns().nullable())
    }
}

impl<Left, Right> AsQuery for LeftOuterJoinSource<Left, Right> where
    SelectStatement<LeftOuterJoinSource<Left, Right>>: Query,
{
    type SqlType = <SelectStatement<Self> as Query>::SqlType;
    type Query = SelectStatement<Self>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl_query_id!(LeftOuterJoinSource<Left, Right>);

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
#[derive(Clone, Copy, Debug)]
pub struct Inner;

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
#[derive(Clone, Copy, Debug)]
pub struct LeftOuter;

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
