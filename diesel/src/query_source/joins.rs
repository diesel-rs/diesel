use prelude::*;
use expression::SelectableExpression;
use expression::nullable::Nullable;
use query_builder::*;
use result::QueryResult;
use super::{QuerySource, Table};
use types::IntoNullable;

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
{
    type FromClause = <Left as JoinTo<Right, Inner>>::JoinClause;

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(Inner)
    }
}

impl<Left, Right> AsQuery for InnerJoinSource<Left, Right> where
    Left: Table + JoinTo<Right, Inner>,
    Right: Table,
    Hlist!(Left::AllColumns, Right::AllColumns): SelectableExpression<
        InnerJoinSource<Left, Right>,
        SqlTypeForSelect=Hlist!(Left::SqlType, Right::SqlType),
    >,
{
    type SqlType = Hlist!(Left::SqlType, Right::SqlType);
    type Query = SelectStatement<
        Hlist!(Left::AllColumns, Right::AllColumns),
        Self,
    >;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(hlist!(Left::all_columns(), Right::all_columns()), self)
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
{
    type FromClause = <Left as JoinTo<Right, LeftOuter>>::JoinClause;

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(LeftOuter)
    }
}

impl<Left, Right> AsQuery for LeftOuterJoinSource<Left, Right> where
    Left: Table + JoinTo<Right, LeftOuter>,
    Right: Table,
    Right::SqlType: IntoNullable,
    Hlist!(Left::AllColumns, Nullable<Right::AllColumns>): SelectableExpression<
        LeftOuterJoinSource<Left, Right>,
        SqlTypeForSelect=Hlist!(Left::SqlType, <Right::SqlType as IntoNullable>::Nullable)
    >,
{
    type SqlType = Hlist!(Left::SqlType, <Right::SqlType as IntoNullable>::Nullable);
    type Query = SelectStatement<
        Hlist!(Left::AllColumns, Nullable<Right::AllColumns>),
        Self,
    >;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(
            hlist!(Left::all_columns(), Right::all_columns().nullable()),
            self,
        )
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
