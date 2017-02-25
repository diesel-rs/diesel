use backend::{Backend, SupportsReturningClause};
use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::*;
use query_source::Table;
use result::QueryResult;

#[derive(Debug)]
pub struct DeleteStatement<T, U>(UpdateTarget<T, U>);

impl<T, U> DeleteStatement<T, U> {
    #[doc(hidden)]
    pub fn new(t: UpdateTarget<T, U>) -> Self {
        DeleteStatement(t)
    }
}

impl<T, U, DB> QueryFragment<DB> for DeleteStatement<T, U> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DELETE FROM ");
        try!(self.0.table.from_clause().to_sql(out));
        try!(self.0.where_clause.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.0.table.from_clause().collect_binds(out));
        try!(self.0.where_clause.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.0.table.from_clause().is_safe_to_cache_prepared() &&
            self.0.where_clause.is_safe_to_cache_prepared()
    }
}

impl_query_id!(noop: DeleteStatement<T, U>);

impl<T, U> AsQuery for DeleteStatement<T, U> where
    T: Table,
    <T as Table>::AllColumns: Expression + SelectableExpression<T>,
    DeleteQuery<<T as Table>::AllColumns, DeleteStatement<T, U>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = DeleteQuery<<T as Table>::AllColumns, DeleteStatement<T, U>>;

    fn as_query(self) -> Self::Query {
        DeleteQuery {
            returning: T::all_columns(),
            statement: self,
        }
    }
}

impl<T, U> DeleteStatement<T, U> {
    /// Specify what expression is returned after execution of the `delete`.
    ///
    /// # Examples
    ///
    /// ### Deleting a record:
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
    /// let deleted_name = diesel::delete(users.filter(name.eq("Sean")))
    ///     .returning(name)
    ///     .get_result(&connection);
    /// assert_eq!(Ok("Sean".to_string()), deleted_name);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> DeleteQuery<E, Self> where
        E: Expression + SelectableExpression<T>,
        DeleteQuery<E, Self>: Query,
    {
        DeleteQuery {
            returning: returns,
            statement: self,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone)]
pub struct DeleteQuery<T, U> {
    returning: T,
    statement: U,
}

impl<T, U> Query for DeleteQuery<T, U> where
    T: Expression + NonAggregate,
{
    type SqlType = T::SqlType;
}

impl<T, U, DB> QueryFragment<DB> for DeleteQuery<T, U> where
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
        self.statement.is_safe_to_cache_prepared() &&
            self.returning.is_safe_to_cache_prepared()
    }
}

impl_query_id!(noop: DeleteQuery<T, U>);
