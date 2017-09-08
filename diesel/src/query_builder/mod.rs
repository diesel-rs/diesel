//! Contains traits responsible for the actual construction of SQL statements
//!
//! The types in this module are part of Diesel's public API, but are generally
//! only useful for implementing Diesel plugins. Applications should generally
//! not need to care about the types inside of this module.

#[macro_use]
mod query_id;
#[macro_use]
mod clause_macro;

mod ast_pass;
pub mod bind_collector;
mod debug_query;
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

pub use self::ast_pass::AstPass;
pub use self::bind_collector::BindCollector;
pub use self::debug_query::DebugQuery;
pub use self::query_id::QueryId;
#[doc(hidden)]
pub use self::select_statement::{BoxedSelectStatement, SelectStatement};
#[doc(inline)]
pub use self::update_statement::{AsChangeset, Changeset, IncompleteUpdateStatement,
                                 IntoUpdateTarget, UpdateStatement, UpdateTarget};
#[doc(inline)]
pub use self::insert_statement::{IncompleteDefaultInsertStatement, IncompleteInsertStatement};

use std::error::Error;

use backend::Backend;
use result::QueryResult;

#[doc(hidden)]
pub type Binds = Vec<Option<Vec<u8>>>;
pub type BuildQueryResult = Result<(), Box<Error + Send + Sync>>;

/// Apps should not need to concern themselves with this trait.
///
/// This is the trait used to actually construct a SQL query. You will take one
/// of these as an argument if you're implementing
/// [`QueryFragment`](trait.QueryFragment.html) manually.
pub trait QueryBuilder<DB: Backend> {
    /// Add `sql` to the end of the query being constructed.
    fn push_sql(&mut self, sql: &str);

    /// Quote `identifier`, and add it to the end of the query being
    /// constructed.
    fn push_identifier(&mut self, identifier: &str) -> QueryResult<()>;

    /// Add a placeholder for a bind parameter to the end of the query being
    /// constructed.
    fn push_bind_param(&mut self);

    /// Returns the constructed SQL query.
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
    /// Walk over this `QueryFragment` for all passes.
    ///
    /// This method is where the actual behavior of an AST node is implemented.
    /// This method will contain the behavior required for all possible AST
    /// passes. See the documentation of [`AstPass`] for more details.
    ///
    /// [`AstPass`]: struct.AstPass.html
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()>;

    /// Converts this `QueryFragment` to its SQL representation
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> QueryResult<()> {
        self.walk_ast(AstPass::to_sql(out))
    }

    /// Serializes all bind parameters in this query.
    ///
    /// A bind parameter is a value which is sent separately from the query
    /// itself. It is represented in SQL with a placeholder such as `?` or `$1`.
    fn collect_binds(
        &self,
        out: &mut DB::BindCollector,
        metadata_lookup: &DB::MetadataLookup,
    ) -> QueryResult<()> {
        self.walk_ast(AstPass::collect_binds(out, metadata_lookup))
    }

    /// Is this query safe to store in the prepared statement cache?
    ///
    /// In order to keep our prepared statement cache at a reasonable size, we
    /// avoid caching any queries which represent a potentially unbounded number
    /// of SQL queries. Generally this will only return `true` for queries for
    /// which `to_sql` will always construct exactly identical SQL.
    ///
    /// Some examples of where this method will return `false` are:
    ///
    /// - `SqlLiteral` (We don't know if the SQL was constructed dynamically, so
    ///   we must assume that it was)
    /// - `In` and `NotIn` (Each value requires a separate bind param
    ///   placeholder)
    fn is_safe_to_cache_prepared(&self) -> QueryResult<bool> {
        let mut result = true;
        self.walk_ast(AstPass::is_safe_to_cache_prepared(&mut result))?;
        Ok(result)
    }
}

impl<T: ?Sized, DB> QueryFragment<DB> for Box<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&**self, pass)
    }
}

impl<'a, T: ?Sized, DB> QueryFragment<DB> for &'a T
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&**self, pass)
    }
}

impl<DB: Backend> QueryFragment<DB> for () {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

/// Types that can be converted into a complete, typed SQL query. This is used
/// internally to automatically add the right select clause when none is
/// specified, or to automatically add `RETURNING *` in certain contexts
pub trait AsQuery {
    /// The SQL type of `Self::Query`
    type SqlType;

    /// What kind of query does this type represent?
    type Query: Query<SqlType = Self::SqlType>;

    /// Converts a type which semantically represents a SQL query into the
    /// actual query being executed. See the trait level docs for more.
    fn as_query(self) -> Self::Query;
}

impl<T: Query> AsQuery for T {
    type SqlType = <Self as Query>::SqlType;
    type Query = Self;

    fn as_query(self) -> Self::Query {
        self
    }
}

/// Takes a query `QueryFragment` expression as an argument and returns a type
/// that implements `fmt::Display` and `fmt::Debug` to show the query.
///
/// The `Display` implementation will show the exact query being sent to the
/// server, with a comment showing the values of the bind parameters. The
/// `Debug` implementation will include the same information in a more
/// structured form, and respects pretty printing.
///
/// # Example
///
/// ### Returning SQL from a count statement:
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// # #[macro_use] extern crate diesel;
/// # use diesel::*;
/// # use schema::*;
/// #
/// # fn main() {
/// #   use schema::users::dsl::*;
/// let sql = debug_query::<DB, _>(&users.count()).to_string();
/// # if cfg!(feature = "postgres") {
/// #     assert_eq!(sql, r#"SELECT COUNT(*) FROM "users" -- binds: []"#);
/// # } else {
/// assert_eq!(sql, "SELECT COUNT(*) FROM `users` -- binds: []");
/// # }
///
/// let query = users.find(1);
/// let debug = debug_query::<DB, _>(&query);
/// # if cfg!(feature = "postgres") {
/// #     assert_eq!(debug.to_string(), "SELECT \"users\".\"id\", \"users\".\"name\" \
/// #         FROM \"users\" WHERE \"users\".\"id\" = $1 -- binds: [1]");
/// # } else {
/// assert_eq!(debug.to_string(), "SELECT `users`.`id`, `users`.`name` FROM `users` \
///     WHERE `users`.`id` = ? -- binds: [1]");
/// # }
///
/// let debug = format!("{:?}", debug);
/// # if !cfg!(feature = "postgres") { // Escaping that string is a pain
/// let expected = "Query { \
///     sql: \"SELECT `users`.`id`, `users`.`name` FROM `users` WHERE \
///         `users`.`id` = ?\", \
///     binds: [1] \
/// }";
/// assert_eq!(debug, expected);
/// # }
/// # }
/// ```
pub fn debug_query<DB, T>(query: &T) -> DebugQuery<T, DB> {
    DebugQuery::new(query)
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", feature = "postgres"))]
pub fn deprecated_debug_sql<T>(query: &T) -> String
where
    T: QueryFragment<::pg::Pg>,
{
    debug_query(query).to_string()
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", feature = "mysql", not(feature = "postgres")))]
pub fn deprecated_debug_sql<T>(query: &T) -> String
where
    T: QueryFragment<::mysql::Mysql>,
{
    debug_query(query).to_string()
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", feature = "sqlite",
            not(any(feature = "postgres", feature = "mysql"))))]
pub fn deprecated_debug_sql<T>(query: &T) -> String
where
    T: QueryFragment<::sqlite::Sqlite>,
{
    debug_query(query).to_string()
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated",
            not(any(feature = "postgres", feature = "mysql", feature = "sqlite"))))]
pub fn deprecated_debug_sql<T>(_query: &T) -> String {
    String::from(
        "At least one backend must be enabled to generated debug SQL",
    )
}
