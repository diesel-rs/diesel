pub mod changeset;
pub mod target;

pub use self::changeset::AsChangeset;
pub use self::target::{IntoUpdateTarget, UpdateTarget};

use backend::Backend;
use dsl::{Filter, IntoBoxed};
use expression::{AppearsOnTable, Expression, NonAggregate, SelectableExpression};
use query_builder::returning_clause::*;
use query_builder::where_clause::*;
use query_builder::*;
use query_dsl::methods::{BoxedDsl, FilterDsl};
use query_dsl::RunQueryDsl;
use query_source::Table;
use result::Error::QueryBuilderError;
use result::QueryResult;

impl<T, U> UpdateStatement<T, U, SetNotCalled> {
    pub(crate) fn new(target: UpdateTarget<T, U>) -> Self {
        UpdateStatement {
            table: target.table,
            where_clause: target.where_clause,
            values: SetNotCalled,
            returning: NoReturningClause,
        }
    }

    /// Provides the `SET` clause of the `UPDATE` statement.
    ///
    /// See [`update`](../fn.update.html) for usage examples, or [the update
    /// guide](https://diesel.rs/guides/all-about-updates/) for a more exhaustive
    /// set of examples.
    pub fn set<V>(self, values: V) -> UpdateStatement<T, U, V::Changeset>
    where
        T: Table,
        V: changeset::AsChangeset<Target = T>,
        UpdateStatement<T, U, V::Changeset>: AsQuery,
    {
        UpdateStatement {
            table: self.table,
            where_clause: self.where_clause,
            values: values.as_changeset(),
            returning: self.returning,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// Represents a complete `UPDATE` statement.
///
/// See [`update`](../fn.update.html) for usage examples, or [the update
/// guide](https://diesel.rs/guides/all-about-updates/) for a more exhaustive
/// set of examples.
pub struct UpdateStatement<T, U, V = SetNotCalled, Ret = NoReturningClause> {
    table: T,
    where_clause: U,
    values: V,
    returning: Ret,
}

/// An `UPDATE` statement with a boxed `WHERE` clause.
pub type BoxedUpdateStatement<'a, DB, T, V = SetNotCalled, Ret = NoReturningClause> =
    UpdateStatement<T, BoxedWhereClause<'a, DB>, V, Ret>;

impl<T, U, V, Ret> UpdateStatement<T, U, V, Ret> {
    /// Adds the given predicate to the `WHERE` clause of the statement being
    /// constructed.
    ///
    /// If there is already a `WHERE` clause, the predicate will be appended
    /// with `AND`. There is no difference in behavior between
    /// `update(table.filter(x))` and `update(table).filter(x)`.
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
    /// let updated_rows = diesel::update(users)
    ///     .set(name.eq("Jim"))
    ///     .filter(name.eq("Sean"))
    ///     .execute(&connection);
    /// assert_eq!(Ok(1), updated_rows);
    ///
    /// let expected_names = vec!["Jim".to_string(), "Tess".to_string()];
    /// let names = users.select(name).order(id).load(&connection);
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

    /// Boxes the `WHERE` clause of this update statement.
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
    /// #     params.insert("tess_has_been_a_jerk", false);
    /// let mut query = diesel::update(users)
    ///     .set(name.eq("Jerk"))
    ///     .into_boxed();
    ///
    /// if !params["tess_has_been_a_jerk"] {
    ///     query = query.filter(name.ne("Tess"));
    /// }
    ///
    /// let updated_rows = query.execute(&connection)?;
    /// assert_eq!(1, updated_rows);
    ///
    /// let expected_names = vec!["Jerk", "Tess"];
    /// let names = users.select(name).order(id).load::<String>(&connection)?;
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

impl<T, U, V, Ret, Predicate> FilterDsl<Predicate> for UpdateStatement<T, U, V, Ret>
where
    U: WhereAnd<Predicate>,
    Predicate: AppearsOnTable<T>,
{
    type Output = UpdateStatement<T, U::Output, V, Ret>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        UpdateStatement {
            table: self.table,
            where_clause: self.where_clause.and(predicate),
            values: self.values,
            returning: self.returning,
        }
    }
}

impl<'a, T, U, V, Ret, DB> BoxedDsl<'a, DB> for UpdateStatement<T, U, V, Ret>
where
    U: Into<BoxedWhereClause<'a, DB>>,
{
    type Output = BoxedUpdateStatement<'a, DB, T, V, Ret>;

    fn internal_into_boxed(self) -> Self::Output {
        UpdateStatement {
            table: self.table,
            where_clause: self.where_clause.into(),
            values: self.values,
            returning: self.returning,
        }
    }
}

impl<T, U, V, Ret, DB> QueryFragment<DB> for UpdateStatement<T, U, V, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
    V: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if self.values.is_noop()? {
            return Err(QueryBuilderError(
                "There are no changes to save. This query cannot be built".into(),
            ));
        }

        out.unsafe_to_cache_prepared();
        out.push_sql("UPDATE ");
        self.table.from_clause().walk_ast(out.reborrow())?;
        out.push_sql(" SET ");
        self.values.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T, U, V, Ret> QueryId for UpdateStatement<T, U, V, Ret> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T, U, V> AsQuery for UpdateStatement<T, U, V, NoReturningClause>
where
    T: Table,
    UpdateStatement<T, U, V, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = UpdateStatement<T, U, V, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<T, U, V, Ret> Query for UpdateStatement<T, U, V, ReturningClause<Ret>>
where
    T: Table,
    Ret: Expression + SelectableExpression<T> + NonAggregate,
{
    type SqlType = Ret::SqlType;
}

impl<T, U, V, Ret, Conn> RunQueryDsl<Conn> for UpdateStatement<T, U, V, Ret> {}

impl<T, U, V> UpdateStatement<T, U, V, NoReturningClause> {
    /// Specify what expression is returned after execution of the `update`.
    /// # Examples
    ///
    /// ### Updating a single record:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let updated_name = diesel::update(users.filter(id.eq(1)))
    ///     .set(name.eq("Dean"))
    ///     .returning(name)
    ///     .get_result(&connection);
    /// assert_eq!(Ok("Dean".to_string()), updated_name);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> UpdateStatement<T, U, V, ReturningClause<E>>
    where
        T: Table,
        UpdateStatement<T, U, V, ReturningClause<E>>: Query,
    {
        UpdateStatement {
            table: self.table,
            where_clause: self.where_clause,
            values: self.values,
            returning: ReturningClause(returns),
        }
    }
}

/// Indicates that you have not yet called `.set` on an update statement
#[derive(Debug, Clone, Copy)]
pub struct SetNotCalled;
