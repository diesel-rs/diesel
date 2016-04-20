pub mod changeset;
pub mod target;

pub use self::changeset::{Changeset, AsChangeset};
pub use self::target::UpdateTarget;

use backend::{Backend, SupportsReturningClause};
use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::{Query, AsQuery, QueryFragment, QueryBuilder, BuildQueryResult};
use query_source::Table;
use result::QueryResult;

/// The type returned by [`update`](fn.update.html). The only thing you can do
/// with this type is call `set` on it.
pub struct IncompleteUpdateStatement<T>(T);

impl<T> IncompleteUpdateStatement<T> {
    #[doc(hidden)]
    pub fn new(t: T) -> Self {
        IncompleteUpdateStatement(t)
    }
}

impl<T: UpdateTarget> IncompleteUpdateStatement<T> {
    pub fn set<U>(self, values: U) -> UpdateStatement<T, U::Changeset> where
        U: changeset::AsChangeset<Target=T::Table>,
        UpdateStatement<T, U::Changeset>: AsQuery,
    {
        UpdateStatement {
            target: self.0,
            values: values.as_changeset(),
        }
    }
}

pub struct UpdateStatement<T, U> {
    target: T,
    values: U,
}

impl<T, U, DB> QueryFragment<DB> for UpdateStatement<T, U> where
    DB: Backend,
    T: UpdateTarget,
    T::WhereClause: QueryFragment<DB>,
    T::FromClause: QueryFragment<DB>,
    U: changeset::Changeset<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("UPDATE ");
        try!(self.target.from_clause().to_sql(out));
        out.push_sql(" SET ");
        try!(self.values.to_sql(out));
        if let Some(clause) = self.target.where_clause() {
            out.push_sql(" WHERE ");
            try!(clause.to_sql(out));
        }
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.target.from_clause().collect_binds(out));
        try!(self.values.collect_binds(out));
        if let Some(clause) = self.target.where_clause() {
            try!(clause.collect_binds(out));
        }
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        false
    }
}

impl_query_id!(noop: UpdateStatement<T, U>);

impl<T, U> AsQuery for UpdateStatement<T, U> where
    T: UpdateTarget,
    UpdateQuery<<T::Table as Table>::AllColumns, UpdateStatement<T, U>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = UpdateQuery<<T::Table as Table>::AllColumns, UpdateStatement<T, U>>;

    fn as_query(self) -> Self::Query {
        UpdateQuery {
            returning: T::Table::all_columns(),
            statement: self,
        }
    }
}

impl<T, U> UpdateStatement<T, U> {
    /// Specify what expression is returned after execution of the `update`.
    /// # Examples
    ///
    /// ### Updating a single record:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
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
    pub fn returning<E>(self, returns: E) -> UpdateQuery<E, UpdateStatement<T, U>> where
        T: UpdateTarget,
        UpdateQuery<E, UpdateStatement<T, U>>: Query,
    {
        UpdateQuery {
            returning: returns,
            statement: self,
        }
    }
}

#[doc(hidden)]
pub struct UpdateQuery<T, U> {
    returning: T,
    statement: U,
}

impl<T, U, V> Query for UpdateQuery<T, UpdateStatement<U, V>> where
    U: UpdateTarget,
    T: Expression + SelectableExpression<U::Table> + NonAggregate,
{
    type SqlType = T::SqlType;
}

impl<T, U, DB> QueryFragment<DB> for UpdateQuery<T, U> where
    DB: Backend + SupportsReturningClause,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.statement.to_sql(out));
        out.push_sql(" RETURNING ");
        try!(self.returning.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.statement.collect_binds(out));
        try!(self.returning.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        false
    }
}

impl_query_id!(noop: UpdateQuery<T, U>);
