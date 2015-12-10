//! Contains traits responsible for the actual construction of SQL statements
#[doc(hidden)]
pub mod pg;
pub mod debug;

mod delete_statement;
mod functions;
mod limit_clause;
mod offset_clause;
mod order_clause;
mod select_statement;
mod where_clause;
pub mod insert_statement;
pub mod update_statement;

pub use self::functions::*;
#[doc(hidden)]
pub use self::select_statement::SelectStatement;
#[doc(inline)]
pub use self::update_statement::{IncompleteUpdateStatement, AsChangeset, Changeset, UpdateTarget};
#[doc(inline)]
pub use self::insert_statement::IncompleteInsertStatement;

use expression::Expression;
use std::error::Error;
use types::NativeSqlType;

#[doc(hidden)]
pub type Binds = Vec<Option<Vec<u8>>>;
pub type BuildQueryResult = Result<(), Box<Error>>;

/// Apps should not need to concern themselves with this trait.
///
/// This is the trait used to actually construct a SQL query. You will take one
/// of these as an argument if you're implementing
/// [`Expression`](../expression/trait.Expression.html) or
/// [`QueryFragment`](trait.QueryFragment.html) manually.
pub trait QueryBuilder {
    fn push_sql(&mut self, sql: &str);
    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult;
    fn push_bound_value(&mut self, tpe: &NativeSqlType, binds: Option<Vec<u8>>);
}

/// A complete SQL query with a return type. This can be a select statement, or
/// a command such as `update` or `insert` with a `RETURNING` clause. Unlike
/// [`Expression`](../expression/trait.Expression.html), types implementing this
/// trait are guaranteed to be executable on their own.
pub trait Query: QueryFragment {
    type SqlType: NativeSqlType;
}

impl<'a, T: Query> Query for &'a T where &'a T: QueryFragment {
    type SqlType = T::SqlType;
}

/// An untyped fragment of SQL. This may be a complete SQL command (such as
/// an update statement without a `RETURNING` clause), or a subsection (such as
/// our internal types used to represent a `WHERE` clause). All methods on
/// [`Connection`](../struct.Connection.html) that execute a query require this
/// trait to be implemented.
pub trait QueryFragment {
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult;
}

impl<T: Expression> QueryFragment for T {
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        Expression::to_sql(self, out)
    }
}

/// Types that can be converted into a complete, typed SQL query. This is used
/// internally to automatically add the right select clause when none is
/// specified, or to automatically add `RETURNING *` in certain contexts
pub trait AsQuery {
    type SqlType: NativeSqlType;
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
