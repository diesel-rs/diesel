pub mod changeset;
pub mod target;

pub use self::changeset::{Changeset, AsChangeset};
pub use self::target::{UpdateTarget, IntoUpdateTarget};

use backend::{Backend, SupportsReturningClause};
use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::{Query, AsQuery, QueryFragment, QueryBuilder, BuildQueryResult};
use query_source::Table;
use result::QueryResult;

/// The type returned by [`update`](/diesel/fn.update.html). The only thing you can do
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
    pub fn set<V>(self, values: V) -> UpdateStatement<T, U, V::Changeset> where
        T: Table,
        V: changeset::AsChangeset<Target=T>,
        UpdateStatement<T, U, V::Changeset>: AsQuery,
    {
        UpdateStatement {
            table: self.0.table,
            where_clause: self.0.where_clause,
            values: values.as_changeset(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UpdateStatement<T, U, V> {
    table: T,
    where_clause: U,
    values: V,
}

impl<T, U, V, DB> QueryFragment<DB> for UpdateStatement<T, U, V> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
    V: changeset::Changeset<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        if self.values.is_noop() {
            return Err("There are no changes to save. This \
                       query cannot be built".into())
        }

        out.push_sql("UPDATE ");
        try!(self.table.from_clause().to_sql(out));
        out.push_sql(" SET ");
        try!(self.values.to_sql(out));
        try!(self.where_clause.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.table.from_clause().collect_binds(out));
        try!(self.values.collect_binds(out));
        try!(self.where_clause.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        false
    }
}

impl_query_id!(noop: UpdateStatement<T, U, V>);

impl<T, U, V> AsQuery for UpdateStatement<T, U, V> where
    T: Table,
    UpdateQuery<T::AllColumns, UpdateStatement<T, U, V>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = UpdateQuery<T::AllColumns, Self>;

    fn as_query(self) -> Self::Query {
        UpdateQuery {
            returning: T::all_columns(),
            statement: self,
        }
    }
}

impl<T, U, V> UpdateStatement<T, U, V> {
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
    pub fn returning<E>(self, returns: E) -> UpdateQuery<E, Self> where
        T: Table,
        UpdateQuery<E, Self>: Query,
    {
        UpdateQuery {
            returning: returns,
            statement: self,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone)]
pub struct UpdateQuery<T, U> {
    returning: T,
    statement: U,
}

impl<Ret, T, U, V> Query for UpdateQuery<Ret, UpdateStatement<T, U, V>> where
    T: Table,
    Ret: Expression + SelectableExpression<T> + NonAggregate,
{
    type SqlType = Ret::SqlType;
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
