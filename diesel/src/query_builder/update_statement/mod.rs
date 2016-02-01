pub mod changeset;
pub mod target;

pub use self::changeset::{Changeset, AsChangeset};
pub use self::target::UpdateTarget;

use backend::{Backend, SupportsReturningClause};
use expression::Expression;
use query_builder::{Query, AsQuery, QueryFragment, QueryBuilder, BuildQueryResult};
use query_source::Table;

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
        UpdateQuery<T, U::Changeset>: Query,
    {
        UpdateStatement {
            target: self.0,
            values: values.as_changeset(),
        }
    }
}

#[doc(hidden)]
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
}

impl<T, U> AsQuery for UpdateStatement<T, U> where
    UpdateQuery<T, U>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = UpdateQuery<T, U>;

    fn as_query(self) -> Self::Query {
        UpdateQuery(self)
    }
}

#[doc(hidden)]
pub struct UpdateQuery<T, U>(UpdateStatement<T, U>);

impl<T, U, DB> QueryFragment<DB> for UpdateQuery<T, U> where
    DB: Backend + SupportsReturningClause,
    T: UpdateTarget,
    <T::Table as Table>::AllColumns: QueryFragment<DB>,
    UpdateStatement<T, U>: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.0.to_sql(out));
        out.push_sql(" RETURNING ");
        try!(T::Table::all_columns().to_sql(out));
        Ok(())
    }
}

impl<T, U> Query for UpdateQuery<T, U> where
    T: UpdateTarget,
{
    type SqlType = <<T::Table as Table>::AllColumns as Expression>::SqlType;
}
