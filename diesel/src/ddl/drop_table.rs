use crate::backend::{Backend, DieselReserveSpecialization};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_dsl::RunQueryDslSupport;
use crate::query_source::Table;
use crate::result::QueryResult;

/// Methods to build table-level DDL statements.
pub trait TableDdl: Table {
    /// Creates a `DROP TABLE` statement for this table.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// # fn main() {
    /// let query = users::table.drop_table();
    /// let sql = diesel::debug_query::<DB, _>(&query).to_string();
    /// # if cfg!(feature = "postgres") {
    /// assert_eq!(r#"DROP TABLE "users" -- binds: []"#, sql);
    /// # } else {
    /// assert_eq!("DROP TABLE `users` -- binds: []", sql);
    /// # }
    /// # }
    /// ```
    fn drop_table(self) -> DropTableStatement<Self> {
        DropTableStatement::new(self)
    }
}

impl<T> TableDdl for T where T: Table {}

/// A `DROP TABLE` statement.
#[must_use = "Queries are only executed when calling `execute` or similar."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTableStatement<T, IfExists = NoIfExists, DropBehavior = NoDropTableBehavior> {
    table: T,
    if_exists: IfExists,
    drop_behavior: DropBehavior,
}

impl<T> DropTableStatement<T> {
    pub(crate) fn new(table: T) -> Self {
        DropTableStatement {
            table,
            if_exists: NoIfExists,
            drop_behavior: NoDropTableBehavior,
        }
    }
}

impl<T, DropBehavior> DropTableStatement<T, NoIfExists, DropBehavior> {
    /// Adds `IF EXISTS` to the statement.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// # fn main() {
    /// let query = users::table.drop_table().if_exists();
    /// let sql = diesel::debug_query::<DB, _>(&query).to_string();
    /// # if cfg!(feature = "postgres") {
    /// assert_eq!(r#"DROP TABLE IF EXISTS "users" -- binds: []"#, sql);
    /// # } else {
    /// assert_eq!("DROP TABLE IF EXISTS `users` -- binds: []", sql);
    /// # }
    /// # }
    /// ```
    pub fn if_exists(self) -> DropTableStatement<T, IfExists, DropBehavior> {
        DropTableStatement {
            table: self.table,
            if_exists: IfExists,
            drop_behavior: self.drop_behavior,
        }
    }
}

impl<T, IfExists> DropTableStatement<T, IfExists, NoDropTableBehavior> {
    /// Adds `CASCADE` to the statement.
    ///
    /// Requires a backend implementing [`SupportsDropTableCascade`], currently PostgreSQL.
    /// The builder has no `DB` parameter, so a MySQL or SQLite statement builds and then
    /// fails at `.execute()`. See
    /// [`DROP TABLE`](https://www.postgresql.org/docs/current/sql-droptable.html).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// let query = users::table.drop_table().if_exists().cascade();
    /// let sql = diesel::debug_query::<DB, _>(&query).to_string();
    /// assert_eq!(r#"DROP TABLE IF EXISTS "users" CASCADE -- binds: []"#, sql);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn cascade(self) -> DropTableStatement<T, IfExists, Cascade> {
        DropTableStatement {
            table: self.table,
            if_exists: self.if_exists,
            drop_behavior: Cascade,
        }
    }
}

impl<T, IfExists, DropBehavior> QueryId for DropTableStatement<T, IfExists, DropBehavior> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T, IfExists, DropBehavior> RunQueryDslSupport
    for DropTableStatement<T, IfExists, DropBehavior>
{
}

impl<T, IfExists, DropBehavior, DB> QueryFragment<DB>
    for DropTableStatement<T, IfExists, DropBehavior>
where
    DB: Backend + DieselReserveSpecialization,
    T: Table + QueryFragment<DB>,
    IfExists: QueryFragment<DB>,
    DropBehavior: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        out.push_sql("DROP TABLE");
        self.if_exists.walk_ast(out.reborrow())?;
        out.push_sql(" ");
        self.table.walk_ast(out.reborrow())?;
        self.drop_behavior.walk_ast(out.reborrow())?;
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoIfExists;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IfExists;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoDropTableBehavior;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cascade;

/// Marker trait for backends that support `DROP TABLE ... CASCADE` semantics.
///
/// Implement it only where the drop really recurses into dependent objects, not where the
/// keyword is accepted and ignored.
#[diagnostic::on_unimplemented(
    message = "`DROP TABLE ... CASCADE` has no cascade semantics for the `{Self}` backend"
)]
pub trait SupportsDropTableCascade {}

#[cfg(feature = "postgres_backend")]
impl SupportsDropTableCascade for crate::pg::Pg {}

impl<DB> QueryFragment<DB> for NoIfExists
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<DB> QueryFragment<DB> for IfExists
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" IF EXISTS");
        Ok(())
    }
}

impl<DB> QueryFragment<DB> for NoDropTableBehavior
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<DB> QueryFragment<DB> for Cascade
where
    DB: Backend + DieselReserveSpecialization + SupportsDropTableCascade,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" CASCADE");
        Ok(())
    }
}
