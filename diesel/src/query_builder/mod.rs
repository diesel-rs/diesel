//! Contains traits responsible for the actual construction of SQL statements
//!
//! The types in this module are part of Diesel's public API, but are generally
//! only useful for implementing Diesel plugins. Applications should generally
//! not need to care about the types inside of this module.

#[macro_use]
mod query_id;
#[macro_use]
mod clause_macro;

#[cfg(diesel_experimental)]
mod aliasing;
mod ast_pass;
pub mod bind_collector;
mod debug_query;
mod delete_statement;
mod distinct_clause;
#[doc(hidden)]
pub mod functions;
mod group_by_clause;
mod insert_statement;
mod limit_clause;
pub(crate) mod locking_clause;
#[doc(hidden)]
pub mod nodes;
mod offset_clause;
mod order_clause;
mod returning_clause;
mod select_clause;
mod select_statement;
mod sql_query;
mod update_statement;
mod where_clause;

#[cfg(diesel_experimental)]
pub use self::aliasing::Aliased;
pub use self::ast_pass::AstPass;
pub use self::bind_collector::BindCollector;
pub use self::debug_query::DebugQuery;
pub use self::delete_statement::{BoxedDeleteStatement, DeleteStatement};
#[doc(inline)]
pub use self::insert_statement::{IncompleteInsertStatement, InsertStatement,
                                 UndecoratedInsertRecord, ValuesClause};
pub use self::query_id::QueryId;
#[doc(hidden)]
pub use self::select_statement::{BoxedSelectStatement, SelectStatement};
pub use self::sql_query::SqlQuery;
#[cfg(feature = "with-deprecated")]
#[allow(deprecated)]
pub use self::update_statement::IncompleteUpdateStatement;
#[doc(inline)]
pub use self::update_statement::{AsChangeset, BoxedUpdateStatement, IntoUpdateTarget,
                                 UpdateStatement, UpdateTarget};

pub(crate) use self::insert_statement::ColumnList;

use std::error::Error;

use backend::Backend;
use result::QueryResult;

#[doc(hidden)]
pub type Binds = Vec<Option<Vec<u8>>>;
/// A specialized Result type used with the query builder.
pub type BuildQueryResult = Result<(), Box<Error + Send + Sync>>;

/// Constructs a SQL query from a Diesel AST.
///
/// The only reason you should ever need to interact with this trait is if you
/// are extending Diesel with support for a new backend. Plugins which extend
/// the query builder with new capabilities will interact with [`AstPass`]
/// instead.
///
/// [`AstPass`]: struct.AstPass.html
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

/// A complete SQL query with a return type.
///
/// This can be a select statement, or a command such as `update` or `insert`
/// with a `RETURNING` clause. Unlike [`Expression`], types implementing this
/// trait are guaranteed to be executable on their own.
///
/// A type which doesn't implement this trait may still represent a complete SQL
/// query. For example, an `INSERT` statement without a `RETURNING` clause will
/// not implement this trait, but can still be executed.
///
/// [`Expression`]: ../expression/trait.Expression.html
pub trait Query {
    /// The SQL type that this query represents.
    ///
    /// This is the SQL type of the `SELECT` clause for select statements, and
    /// the SQL type of the `RETURNING` clause for insert, update, or delete
    /// statements.
    type SqlType;
}

impl<'a, T: Query> Query for &'a T {
    type SqlType = T::SqlType;
}

/// Indicates that a type is a `SELECT` statement.
///
/// This trait differs from `Query` in two ways:
/// - It is implemented only for select statements, rather than all queries
///   which return a value.
/// - It has looser constraints. A type implementing `SelectQuery` is known to
///   be potentially valid if used as a subselect, but it is not necessarily
///   able to be executed.
pub trait SelectQuery {
    /// The SQL type of the `SELECT` clause
    type SqlType;
}

/// An untyped fragment of SQL.
///
/// This may be a complete SQL command (such as an update statement without a
/// `RETURNING` clause), or a subsection (such as our internal types used to
/// represent a `WHERE` clause). Implementations of [`ExecuteDsl`] and
/// [`LoadQuery`] will generally require that this trait be implemented.
///
/// [`ExecuteDsl`]: ../query_dsl/methods/trait.ExecuteDsl.html
/// [`LoadQuery`]: ../query_dsl/methods/trait.LoadQuery.html
pub trait QueryFragment<DB: Backend> {
    /// Walk over this `QueryFragment` for all passes.
    ///
    /// This method is where the actual behavior of an AST node is implemented.
    /// This method will contain the behavior required for all possible AST
    /// passes. See [`AstPass`] for more details.
    ///
    /// [`AstPass`]: struct.AstPass.html
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()>;

    /// Converts this `QueryFragment` to its SQL representation.
    ///
    /// This method should only be called by implementations of `Connection`.
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> QueryResult<()> {
        self.walk_ast(AstPass::to_sql(out))
    }

    /// Serializes all bind parameters in this query.
    ///
    /// A bind parameter is a value which is sent separately from the query
    /// itself. It is represented in SQL with a placeholder such as `?` or `$1`.
    ///
    /// This method should only be called by implementations of `Connection`.
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
    ///
    /// This method should only be called by implementations of `Connection`.
    fn is_safe_to_cache_prepared(&self) -> QueryResult<bool> {
        let mut result = true;
        self.walk_ast(AstPass::is_safe_to_cache_prepared(&mut result))?;
        Ok(result)
    }

    #[doc(hidden)]
    /// Does walking this AST have any effect?
    fn is_noop(&self) -> QueryResult<bool> {
        let mut result = true;
        self.walk_ast(AstPass::is_noop(&mut result))?;
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

impl<T, DB> QueryFragment<DB> for Option<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, out: AstPass<DB>) -> QueryResult<()> {
        match *self {
            Some(ref c) => c.walk_ast(out),
            None => Ok(()),
        }
    }
}

/// Types that can be converted into a complete, typed SQL query.
///
/// This is used internally to automatically add the right select clause when
/// none is specified, or to automatically add `RETURNING *` in certain contexts.
///
/// A type which implements this trait is guaranteed to be valid for execution.
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
