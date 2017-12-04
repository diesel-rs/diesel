pub mod changeset;
pub mod target;

pub use self::changeset::{AsChangeset, Changeset};
pub use self::target::{IntoUpdateTarget, UpdateTarget};

use backend::Backend;
use dsl::Filter;
use expression::{AppearsOnTable, Expression, NonAggregate, SelectableExpression};
use query_builder::*;
use query_builder::returning_clause::*;
use query_builder::where_clause::*;
use query_dsl::RunQueryDsl;
use query_dsl::methods::FilterDsl;
use query_source::Table;
use result::Error::QueryBuilderError;
use result::QueryResult;

/// The type returned by [`update`](../fn.update.html). The only thing you can do
/// with this type is call `set` on it.
#[derive(Debug)]
pub struct IncompleteUpdateStatement<T, U>(UpdateTarget<T, U>);

impl<T, U> IncompleteUpdateStatement<T, U> {
    #[doc(hidden)]
    pub fn new(t: UpdateTarget<T, U>) -> Self {
        IncompleteUpdateStatement(t)
    }
}

impl<T, U> IncompleteUpdateStatement<T, U> {
    /// Provides the `SET` clause of the `UPDATE` statement.
    ///
    /// See [`update`](../fn.update.html) for usage examples, or [the update
    /// guide](https://diesel.rs/guides/all-about-updates/) for a more exhaustive
    /// set of examples.
    pub fn set<V>(self, values: V) -> UpdateStatement<T, U, V::Changeset, NoReturningClause>
    where
        T: Table,
        V: changeset::AsChangeset<Target = T>,
        UpdateStatement<T, U, V::Changeset, NoReturningClause>: AsQuery,
    {
        UpdateStatement {
            table: self.0.table,
            where_clause: self.0.where_clause,
            values: values.as_changeset(),
            returning: NoReturningClause,
        }
    }

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
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use users::dsl::*;
    /// #     let connection = establish_connection();
    /// let updated_rows = diesel::update(users)
    ///     .filter(name.eq("Sean"))
    ///     .set(name.eq("Jim"))
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
}

impl<T, U, Predicate> FilterDsl<Predicate> for IncompleteUpdateStatement<T, U>
where
    U: WhereAnd<Predicate>,
    Predicate: AppearsOnTable<T>,
{
    type Output = IncompleteUpdateStatement<T, U::Output>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        IncompleteUpdateStatement::new(UpdateTarget {
            table: self.0.table,
            where_clause: self.0.where_clause.and(predicate),
        })
    }
}

#[derive(Debug, Copy, Clone)]
/// Represents a complete `UPDATE` statement.
///
/// See [`update`](../fn.update.html) for usage examples, or [the update
/// guide](https://diesel.rs/guides/all-about-updates/) for a more exhaustive
/// set of examples.
pub struct UpdateStatement<T, U, V, Ret = NoReturningClause> {
    table: T,
    where_clause: U,
    values: V,
    returning: Ret,
}

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
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use users::dsl::*;
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

impl<T, U, V, Ret, DB> QueryFragment<DB> for UpdateStatement<T, U, V, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
    V: changeset::Changeset<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if self.values.is_noop() {
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

impl_query_id!(noop: UpdateStatement<T, U, V, Ret>);

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
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use self::users::dsl::*;
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
