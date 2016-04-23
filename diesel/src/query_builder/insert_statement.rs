use backend::{Backend, SupportsReturningClause};
use expression::{Expression, SelectableExpression, NonAggregate};
use persistable::{Insertable, InsertValues};
use query_builder::*;
use query_source::Table;
use result::QueryResult;

/// The structure returned by [`insert`](fn.insert.html). The only thing that can be done with it
/// is call `into`.
pub struct IncompleteInsertStatement<T, Op> {
    records: T,
    operator: Op
}

impl<T, Op> IncompleteInsertStatement<T, Op> {
    #[doc(hidden)]
    pub fn new(records: T, operator: Op) -> Self {
        IncompleteInsertStatement {
            records: records,
            operator: operator,
        }
    }

    /// Specify which table the data passed to `insert` should be added to.
    pub fn into<S>(self, target: S) -> InsertStatement<S, T, Op> where
        InsertStatement<S, T, Op>: AsQuery,
    {
        InsertStatement {
            operator: self.operator,
            target: target,
            records: self.records,
        }
    }
}

pub struct InsertStatement<T, U, Op> {
    operator: Op,
    target: T,
    records: U,
}

impl<T, U, Op, DB> QueryFragment<DB> for InsertStatement<T, U, Op> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: Insertable<T, DB> + Copy,
    Op: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        let values = self.records.values();
        try!(self.operator.to_sql(out));
        out.push_sql(" INTO ");
        try!(self.target.from_clause().to_sql(out));
        out.push_sql(" (");
        try!(values.column_names(out));
        out.push_sql(") VALUES ");
        try!(values.values_clause(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        let values = self.records.values();
        try!(self.operator.collect_binds(out));
        try!(self.target.from_clause().collect_binds(out));
        try!(values.values_bind_params(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        false
    }
}

impl_query_id!(noop: InsertStatement<T, U, Op>);

impl<T, U, Op> AsQuery for InsertStatement<T, U, Op> where
    T: Table,
    InsertQuery<T::AllColumns, InsertStatement<T, U, Op>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = InsertQuery<T::AllColumns, Self>;

    fn as_query(self) -> Self::Query {
        InsertQuery {
            returning: T::all_columns(),
            statement: self,
        }
    }
}

impl<T, U, Op> InsertStatement<T, U, Op> {
    /// Specify what expression is returned after execution of the `insert`.
    /// # Examples
    ///
    /// ### Inserting a record:
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
    /// let new_user = NewUser {
    ///     name: "Timmy".to_string(),
    /// };
    ///
    /// let inserted_name = diesel::insert(&new_user)
    ///     .into(users)
    ///     .returning(name)
    ///     .get_result(&connection);
    /// assert_eq!(Ok("Timmy".to_string()), inserted_name);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> InsertQuery<E, Self> where
        E: Expression + SelectableExpression<T>,
        InsertQuery<E, Self>: Query,
    {
        InsertQuery {
            returning: returns,
            statement: self,
        }
    }
}

#[doc(hidden)]
pub struct InsertQuery<T, U> {
    returning: T,
    statement: U,
}

impl<T, U> Query for InsertQuery<T, U> where
    T: Expression + NonAggregate,
{
    type SqlType = T::SqlType;
}

impl<T, U, DB> QueryFragment<DB> for InsertQuery<T, U> where
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

impl_query_id!(noop: InsertQuery<T, U>);

pub struct Insert;

impl<DB: Backend> QueryFragment<DB> for Insert {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("INSERT");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl_query_id!(Insert);
