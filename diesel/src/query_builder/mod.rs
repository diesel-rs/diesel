//! Contains traits responsible for the actual construction of SQL statements
//!
//! The types in this module are part of Diesel's public API, but are generally
//! only useful for implementing Diesel plugins. Applications should generally
//! not need to care about the types inside of this module.

#[macro_use]
mod query_id;
#[macro_use]
mod clause_macro;

pub(crate) mod ast_pass;
pub mod bind_collector;
mod collected_query;
pub(crate) mod combination_clause;
mod debug_query;
mod delete_statement;
mod distinct_clause;
pub(crate) mod from_clause;
pub(crate) mod functions;
mod group_by_clause;
mod having_clause;
pub(crate) mod insert_statement;
pub(crate) mod limit_clause;
pub(crate) mod limit_offset_clause;
pub(crate) mod locking_clause;
pub(crate) mod nodes;
pub(crate) mod offset_clause;
pub(crate) mod order_clause;
pub(crate) mod returning_clause;
pub(crate) mod select_clause;
pub(crate) mod select_statement;
mod sql_query;
pub(crate) mod update_statement;
pub(crate) mod upsert;
pub(crate) mod where_clause;

#[doc(inline)]
pub use self::ast_pass::AstPass;
#[doc(inline)]
pub use self::bind_collector::{BindCollector, MoveableBindCollector};
#[doc(inline)]
pub use self::collected_query::CollectedQuery;
#[doc(inline)]
pub use self::debug_query::DebugQuery;
#[doc(inline)]
pub use self::delete_statement::{BoxedDeleteStatement, DeleteStatement};
#[doc(inline)]
pub use self::insert_statement::{
    IncompleteInsertOrIgnoreStatement, IncompleteInsertStatement, IncompleteReplaceStatement,
    InsertOrIgnoreStatement, InsertStatement, ReplaceStatement,
};
#[doc(inline)]
pub use self::query_id::QueryId;
#[doc(inline)]
pub use self::sql_query::{BoxedSqlQuery, SqlQuery};
#[doc(inline)]
pub use self::upsert::on_conflict_target_decorations::DecoratableTarget;

#[doc(inline)]
pub use self::update_statement::changeset::AsChangeset;
#[doc(inline)]
pub use self::update_statement::target::{IntoUpdateTarget, UpdateTarget};
#[doc(inline)]
pub use self::update_statement::{BoxedUpdateStatement, UpdateStatement};

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::limit_clause::{LimitClause, NoLimitClause};
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::offset_clause::{NoOffsetClause, OffsetClause};

#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
#[doc(inline)]
pub(crate) use self::insert_statement::batch_insert::BatchInsert;
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::insert_statement::{UndecoratedInsertRecord, ValuesClause};

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
#[doc(inline)]
pub use self::insert_statement::DefaultValues;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
#[doc(inline)]
pub use self::returning_clause::ReturningClause;

#[doc(inline)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::ast_pass::AstPassToSqlOptions;

#[doc(inline)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::select_clause::SelectClauseExpression;

#[doc(inline)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::from_clause::{FromClause, NoFromClause};
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
#[doc(inline)]
pub(crate) use self::select_statement::BoxedSelectStatement;

#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
#[doc(inline)]
pub(crate) use self::select_statement::SelectStatement;

pub(crate) use self::insert_statement::ColumnList;

#[cfg(feature = "postgres_backend")]
pub use crate::pg::query_builder::only::Only;

#[cfg(feature = "postgres_backend")]
pub use crate::pg::query_builder::tablesample::{Tablesample, TablesampleMethod};

#[cfg(feature = "postgres_backend")]
pub(crate) use self::bind_collector::ByteWrapper;
use crate::backend::Backend;
use crate::result::QueryResult;
use std::error::Error;

#[doc(hidden)]
pub type Binds = Vec<Option<Vec<u8>>>;
/// A specialized Result type used with the query builder.
pub type BuildQueryResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Constructs a SQL query from a Diesel AST.
///
/// The only reason you should ever need to interact with this trait is if you
/// are extending Diesel with support for a new backend. Plugins which extend
/// the query builder with new capabilities will interact with [`AstPass`]
/// instead.
///
pub trait QueryBuilder<DB: Backend> {
    /// Add `sql` to the end of the query being constructed.
    fn push_sql(&mut self, sql: &str);

    /// Quote `identifier`, and add it to the end of the query being
    /// constructed.
    fn push_identifier(&mut self, identifier: &str) -> QueryResult<()>;

    /// Add a placeholder for a bind parameter to the end of the query being
    /// constructed.
    fn push_bind_param(&mut self);

    /// Increases the internal counter for bind parameters without adding the
    /// bind parameter itself to the query
    fn push_bind_param_value_only(&mut self) {}

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
/// [`Expression`]: crate::expression::Expression
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
/// [`ExecuteDsl`]: crate::query_dsl::methods::ExecuteDsl
/// [`LoadQuery`]: crate::query_dsl::methods::LoadQuery
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no valid SQL fragment for the `{DB}` backend",
    note = "this usually means that the `{DB}` database system does not support \n\
            this SQL syntax"
)]
pub trait QueryFragment<DB: Backend, SP = self::private::NotSpecialized> {
    /// Walk over this `QueryFragment` for all passes.
    ///
    /// This method is where the actual behavior of an AST node is implemented.
    /// This method will contain the behavior required for all possible AST
    /// passes. See [`AstPass`] for more details.
    ///
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()>;

    /// Converts this `QueryFragment` to its SQL representation.
    ///
    /// This method should only be called by implementations of `Connection`.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn to_sql(&self, out: &mut DB::QueryBuilder, backend: &DB) -> QueryResult<()> {
        let mut options = AstPassToSqlOptions::default();
        self.walk_ast(AstPass::to_sql(out, &mut options, backend))
    }

    /// Serializes all bind parameters in this query.
    ///
    /// A bind parameter is a value which is sent separately from the query
    /// itself. It is represented in SQL with a placeholder such as `?` or `$1`.
    ///
    /// This method should only be called by implementations of `Connection`.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn collect_binds<'b>(
        &'b self,
        out: &mut DB::BindCollector<'b>,
        metadata_lookup: &mut DB::MetadataLookup,
        backend: &'b DB,
    ) -> QueryResult<()> {
        self.walk_ast(AstPass::collect_binds(out, metadata_lookup, backend))
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
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn is_safe_to_cache_prepared(&self, backend: &DB) -> QueryResult<bool> {
        let mut result = true;
        self.walk_ast(AstPass::is_safe_to_cache_prepared(&mut result, backend))?;
        Ok(result)
    }

    /// Does walking this AST have any effect?
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn is_noop(&self, backend: &DB) -> QueryResult<bool> {
        let mut result = true;
        self.walk_ast(AstPass::is_noop(&mut result, backend))?;
        Ok(result)
    }
}

impl<T: ?Sized, DB> QueryFragment<DB> for Box<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&**self, pass)
    }
}

impl<'a, T: ?Sized, DB> QueryFragment<DB> for &'a T
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&**self, pass)
    }
}

impl<DB: Backend> QueryFragment<DB> for () {
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<T, DB> QueryFragment<DB> for Option<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        match *self {
            Some(ref c) => c.walk_ast(out),
            None => Ok(()),
        }
    }
}

/// A trait used to construct type erased boxed variant of the current query node
///
/// Mainly useful for implementing third party backends
pub trait IntoBoxedClause<'a, DB> {
    /// Resulting type
    type BoxedClause;

    /// Convert the given query node in it's boxed representation
    fn into_boxed(self) -> Self::BoxedClause;
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
    // This method is part of our public API,
    // so we won't change the name to just appease clippy
    // (Also the trait is literally named `AsQuery` so
    // naming the method similarity is fine)
    #[allow(clippy::wrong_self_convention)]
    fn as_query(self) -> Self::Query;
}

impl<T: Query> AsQuery for T {
    type SqlType = <T as Query>::SqlType;
    type Query = T;

    fn as_query(self) -> <T as AsQuery>::Query {
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
/// #         FROM \"users\" WHERE (\"users\".\"id\" = $1) -- binds: [1]");
/// # } else {
/// assert_eq!(debug.to_string(), "SELECT `users`.`id`, `users`.`name` FROM `users` \
///     WHERE (`users`.`id` = ?) -- binds: [1]");
/// # }
///
/// let debug = format!("{:?}", debug);
/// # if !cfg!(feature = "postgres") { // Escaping that string is a pain
/// let expected = "Query { \
///     sql: \"SELECT `users`.`id`, `users`.`name` FROM `users` WHERE \
///         (`users`.`id` = ?)\", \
///     binds: [1] \
/// }";
/// assert_eq!(debug, expected);
/// # }
/// # }
/// ```
pub fn debug_query<DB, T>(query: &T) -> DebugQuery<'_, T, DB> {
    DebugQuery::new(query)
}

mod private {
    #[allow(missing_debug_implementations, missing_copy_implementations)]
    pub struct NotSpecialized;
}
