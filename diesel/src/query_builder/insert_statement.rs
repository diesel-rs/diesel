use backend::{Backend, SupportsReturningClause};
use persistable::{Insertable, InsertableColumns};
use expression::Expression;
use query_builder::*;
use query_source::Table;

/// The structure returned by [`insert`](fn.insert.html). The only thing that can be done with it
/// is call `into`.
pub struct IncompleteInsertStatement<T> {
    records: T,
}

impl<T> IncompleteInsertStatement<T> {
    #[doc(hidden)]
    pub fn new(records: T) -> Self {
        IncompleteInsertStatement {
            records: records,
        }
    }

    /// Specify which table the data passed to `insert` should be added to.
    pub fn into<S>(self, target: S) -> InsertStatement<S, T> where
        InsertStatement<S, T>: AsQuery,
    {
        InsertStatement {
            target: target,
            records: self.records,
        }
    }
}

#[doc(hidden)]
pub struct InsertStatement<T, U> {
    target: T,
    records: U,
}

impl<T, U, DB> QueryFragment<DB> for InsertStatement<T, U> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: Insertable<T, DB> + Copy,
    U::Values: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_context(Context::Insert);
        out.push_sql("INSERT INTO ");
        try!(self.target.from_clause().to_sql(out));
        out.push_sql(" (");
        out.push_sql(&U::columns().names());
        out.push_sql(") VALUES ");
        try!(QueryFragment::to_sql(&self.records.values(), out));
        out.pop_context();
        Ok(())
    }
}

impl<T, U> AsQuery for InsertStatement<T, U> where
    T: Table,
    InsertQuery<T::AllColumns, InsertStatement<T, U>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = InsertQuery<T::AllColumns, InsertStatement<T, U>>;

    fn as_query(self) -> Self::Query {
        InsertQuery {
            returning: T::all_columns(),
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
    T: Expression,
{
    type SqlType = T::SqlType;
}

impl<T, U, DB> QueryFragment<DB> for InsertQuery<T, U> where
    DB: Backend + SupportsReturningClause,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_context(Context::Insert);
        try!(self.statement.to_sql(out));
        out.push_sql(" RETURNING ");
        try!(self.returning.to_sql(out));
        out.pop_context();
        Ok(())
    }
}
