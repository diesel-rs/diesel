//! Contains traits responsible for the actual construction of SQL statements
pub mod debug;

#[macro_use]
mod query_id;
#[macro_use]
mod clause_macro;

pub mod bind_collector;
mod delete_statement;
#[doc(hidden)]
pub mod functions;
#[doc(hidden)]
pub mod nodes;
mod distinct_clause;
mod group_by_clause;
mod limit_clause;
mod offset_clause;
mod order_clause;
mod returning_clause;
mod select_clause;
mod select_statement;
pub mod where_clause;
pub mod insert_statement;
pub mod update_statement;

pub use self::bind_collector::BindCollector;
pub use self::query_id::QueryId;
#[doc(hidden)]
pub use self::select_statement::{SelectStatement, BoxedSelectStatement};
#[doc(inline)]
pub use self::update_statement::{
    AsChangeset,
    Changeset,
    IncompleteUpdateStatement,
    IntoUpdateTarget,
    UpdateStatement,
    UpdateTarget,
};
#[doc(inline)]
pub use self::insert_statement::IncompleteInsertStatement;

use std::error::Error;

use backend::Backend;
use result::QueryResult;

#[doc(hidden)]
pub type Binds = Vec<Option<Vec<u8>>>;
pub type BuildQueryResult = Result<(), Box<Error+Send+Sync>>;

/// Apps should not need to concern themselves with this trait.
///
/// This is the trait used to actually construct a SQL query. You will take one
/// of these as an argument if you're implementing
/// [`QueryFragment`](trait.QueryFragment.html) manually.
pub trait QueryBuilder<DB: Backend> {
    fn push_sql(&mut self, sql: &str);
    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult;
    fn push_bind_param(&mut self);
    fn finish(self) -> String;
}

/// A complete SQL query with a return type. This can be a select statement, or
/// a command such as `update` or `insert` with a `RETURNING` clause. Unlike
/// [`Expression`](../expression/trait.Expression.html), types implementing this
/// trait are guaranteed to be executable on their own.
pub trait Query {
    type SqlType;
}

impl<'a, T: Query> Query for &'a T {
    type SqlType = T::SqlType;
}

/// An untyped fragment of SQL. This may be a complete SQL command (such as
/// an update statement without a `RETURNING` clause), or a subsection (such as
/// our internal types used to represent a `WHERE` clause). All methods on
/// [`Connection`](../connection/trait.Connection.html) that execute a query require this
/// trait to be implemented.
pub trait QueryFragment<DB: Backend> {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()>;
    fn is_safe_to_cache_prepared(&self) -> bool;
}

impl<T: ?Sized, DB> QueryFragment<DB> for Box<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        QueryFragment::to_sql(&**self, out)
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        QueryFragment::collect_binds(&**self, out)
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        QueryFragment::is_safe_to_cache_prepared(&**self)
    }
}

impl<'a, T: ?Sized, DB> QueryFragment<DB> for &'a T where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        QueryFragment::to_sql(&**self, out)
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        QueryFragment::collect_binds(&**self, out)
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        QueryFragment::is_safe_to_cache_prepared(&**self)
    }
}

impl<DB: Backend> QueryFragment<DB> for () {
    fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

/// Types that can be converted into a complete, typed SQL query. This is used
/// internally to automatically add the right select clause when none is
/// specified, or to automatically add `RETURNING *` in certain contexts
pub trait AsQuery {
    type SqlType;
    type Query: Query<SqlType=Self::SqlType>;

    fn as_query(self) -> Self::Query;
}

impl<T: Query> AsQuery for T {
    type SqlType = <Self as Query>::SqlType;
    type Query = Self;

    fn as_query(self) -> Self::Query {
        self
    }
}
