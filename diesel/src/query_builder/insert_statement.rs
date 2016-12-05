use backend::Backend;
use expression::{Expression, SelectableExpression, NonAggregate};
use persistable::{Insertable, InsertValues};
use query_builder::*;
use query_source::Table;
use result::QueryResult;
use super::returning_clause::*;

/// The structure returned by [`insert`](fn.insert.html). The only thing that can be done with it
/// is call `into`.
#[derive(Debug, Copy, Clone)]
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
            returning: NoReturningClause,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InsertStatement<T, U, Op, Ret=NoReturningClause> {
    operator: Op,
    target: T,
    records: U,
    returning: Ret,
}

impl<T, U, Op, Ret, DB> QueryFragment<DB> for InsertStatement<T, U, Op, Ret> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: Insertable<T, DB> + Copy,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
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
        try!(self.returning.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        let values = self.records.values();
        try!(self.operator.collect_binds(out));
        try!(self.target.from_clause().collect_binds(out));
        try!(values.values_bind_params(out));
        try!(self.returning.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        false
    }
}

impl_query_id!(noop: InsertStatement<T, U, Op, Ret>);

impl<T, U, Op> AsQuery for InsertStatement<T, U, Op, NoReturningClause> where
    T: Table,
    InsertStatement<T, U, Op, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = InsertStatement<T, U, Op, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<T, U, Op, Ret> Query for InsertStatement<T, U, Op, ReturningClause<Ret>> where
    Ret: Expression + SelectableExpression<T> + NonAggregate,
{
    type SqlType = Ret::SqlType;
}

impl<T, U, Op> InsertStatement<T, U, Op, NoReturningClause> {
    /// Specify what expression is returned after execution of the `insert`.
    /// This method can only be called once.
    ///
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
    pub fn returning<E>(self, returns: E)
        -> InsertStatement<T, U, Op, ReturningClause<E>> where
            InsertStatement<T, U, Op, ReturningClause<E>>: Query,
    {
        InsertStatement {
            operator: self.operator,
            target: self.target,
            records: self.records,
            returning: ReturningClause(returns),
        }
    }
}

#[derive(Debug, Copy, Clone)]
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
