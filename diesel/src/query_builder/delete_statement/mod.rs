use crate::backend::Backend;
use crate::dsl::{Filter, IntoBoxed};
use crate::expression::{AppearsOnTable, SelectableExpression};
use crate::query_builder::returning_clause::*;
use crate::query_builder::where_clause::*;
use crate::query_builder::*;
use crate::query_dsl::methods::{BoxedDsl, FilterDsl};
use crate::query_dsl::RunQueryDsl;
use crate::query_source::Table;
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// Represents a SQL `DELETE` statement.
///
/// The type parameters on this struct represent:
///
/// - `T`: The table we are deleting from.
/// - `U`: The `WHERE` clause of this query. The exact types used to represent
///   this are private, and you should not make any assumptions about them.
/// - `Ret`: The `RETURNING` clause of this query. The exact types used to
///   represent this are private. You can safely rely on the default type
///   representing the lack of a `RETURNING` clause.
pub struct DeleteStatement<T, U, Ret = NoReturningClause> {
    table: T,
    where_clause: U,
    returning: Ret,
}

/// A `DELETE` statement with a boxed `WHERE` clause
pub type BoxedDeleteStatement<'a, DB, T, Ret = NoReturningClause> =
    DeleteStatement<T, BoxedWhereClause<'a, DB>, Ret>;

impl<T, U> DeleteStatement<T, U, NoReturningClause> {
    pub(crate) fn new(table: T, where_clause: U) -> Self {
        DeleteStatement {
            table: table,
            where_clause: where_clause,
            returning: NoReturningClause,
        }
    }

    /// Adds the given predicate to the `WHERE` clause of the statement being
    /// constructed.
    ///
    /// If there is already a `WHERE` clause, the predicate will be appended
    /// with `AND`. There is no difference in behavior between
    /// `delete(table.filter(x))` and `delete(table).filter(x)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let deleted_rows = diesel::delete(users)
    ///     .filter(name.eq("Sean"))
    ///     .execute(&connection);
    /// assert_eq!(Ok(1), deleted_rows);
    ///
    /// let expected_names = vec!["Tess".to_string()];
    /// let names = users.select(name).load(&connection);
    ///
    /// assert_eq!(Ok(expected_names), names);
    /// # }
    /// ```
    pub fn filter<Predicate>(self, predicate: Predicate) -> Filter<Self, Predicate>
    where
        Self: FilterDsl<Predicate>,
    {
        FilterDsl::filter(self, predicate)
    }

    /// Boxes the `WHERE` clause of this delete statement.
    ///
    /// This is useful for cases where you want to conditionally modify a query,
    /// but need the type to remain the same. The backend must be specified as
    /// part of this. It is not possible to box a query and have it be useable
    /// on multiple backends.
    ///
    /// A boxed query will incur a minor performance penalty, as the query builder
    /// can no longer be inlined by the compiler. For most applications this cost
    /// will be minimal.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::HashMap;
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     let mut params = HashMap::new();
    /// #     params.insert("sean_has_been_a_jerk", true);
    /// let mut query = diesel::delete(users)
    ///     .into_boxed();
    ///
    /// if params["sean_has_been_a_jerk"] {
    ///     query = query.filter(name.eq("Sean"));
    /// }
    ///
    /// let deleted_rows = query.execute(&connection)?;
    /// assert_eq!(1, deleted_rows);
    ///
    /// let expected_names = vec!["Tess"];
    /// let names = users.select(name).load::<String>(&connection)?;
    ///
    /// assert_eq!(expected_names, names);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn into_boxed<'a, DB>(self) -> IntoBoxed<'a, Self, DB>
    where
        DB: Backend,
        Self: BoxedDsl<'a, DB>,
    {
        BoxedDsl::internal_into_boxed(self)
    }
}

impl<T, U, Ret, Predicate> FilterDsl<Predicate> for DeleteStatement<T, U, Ret>
where
    U: WhereAnd<Predicate>,
    Predicate: AppearsOnTable<T>,
{
    type Output = DeleteStatement<T, U::Output, Ret>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        DeleteStatement {
            table: self.table,
            where_clause: self.where_clause.and(predicate),
            returning: self.returning,
        }
    }
}

impl<'a, T, U, Ret, DB> BoxedDsl<'a, DB> for DeleteStatement<T, U, Ret>
where
    U: Into<BoxedWhereClause<'a, DB>>,
{
    type Output = BoxedDeleteStatement<'a, DB, T, Ret>;

    fn internal_into_boxed(self) -> Self::Output {
        DeleteStatement {
            table: self.table,
            where_clause: self.where_clause.into(),
            returning: self.returning,
        }
    }
}

impl<T, U, Ret, DB> QueryFragment<DB> for DeleteStatement<T, U, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("DELETE FROM ");
        self.table.from_clause().walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T, U> AsQuery for DeleteStatement<T, U, NoReturningClause>
where
    T: Table,
    T::AllColumns: SelectableExpression<T>,
    DeleteStatement<T, U, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = DeleteStatement<T, U, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<T, U, Ret> Query for DeleteStatement<T, U, ReturningClause<Ret>>
where
    T: Table,
    Ret: SelectableExpression<T>,
{
    type SqlType = Ret::SqlType;
}

impl<T, U, Ret, Conn> RunQueryDsl<Conn> for DeleteStatement<T, U, Ret> {}

impl<T, U> DeleteStatement<T, U, NoReturningClause> {
    /// Specify what expression is returned after execution of the `delete`.
    ///
    /// # Examples
    ///
    /// ### Deleting a record:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let deleted_name = diesel::delete(users.filter(name.eq("Sean")))
    ///     .returning(name)
    ///     .get_result(&connection);
    /// assert_eq!(Ok("Sean".to_string()), deleted_name);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> DeleteStatement<T, U, ReturningClause<E>>
    where
        E: SelectableExpression<T>,
        DeleteStatement<T, U, ReturningClause<E>>: Query,
    {
        DeleteStatement {
            table: self.table,
            where_clause: self.where_clause,
            returning: ReturningClause(returns),
        }
    }
}
