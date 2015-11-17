pub mod changeset;
pub mod target;

pub use self::changeset::{Changeset, AsChangeset};
pub use self::target::UpdateTarget;

use expression::Expression;
use query_builder::{Query, AsQuery, QueryFragment, QueryBuilder, BuildQueryResult};
use query_source::Table;

pub fn update<T: UpdateTarget>(source: T) -> IncompleteUpdateStatement<T> {
    IncompleteUpdateStatement(source)
}

pub struct IncompleteUpdateStatement<T>(T);

impl<T> IncompleteUpdateStatement<T> {
    pub fn set<U>(self, values: U) -> UpdateStatement<T, U::Changeset> where
        U: changeset::AsChangeset,
        UpdateStatement<T, U::Changeset>: QueryFragment,
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

impl<T, U> QueryFragment for UpdateStatement<T, U> where
    T: UpdateTarget,
    U: changeset::Changeset<Target=T::Table>,
{
    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("UPDATE ");
        try!(self.target.from_clause(out));
        out.push_sql(" SET ");
        try!(self.values.to_sql(out));
        self.target.where_clause(out)
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

pub struct UpdateQuery<T, U>(UpdateStatement<T, U>);

impl<T, U> QueryFragment for UpdateQuery<T, U> where
    UpdateStatement<T, U>: QueryFragment,
{
    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        try!(self.0.to_sql(out));
        out.push_sql(" RETURNING *");
        Ok(())
    }
}

impl<T, U> Query for UpdateQuery<T, U> where
    UpdateQuery<T, U>: QueryFragment,
    T: UpdateTarget,
{
    type SqlType = <<T::Table as Table>::Star as Expression>::SqlType;
}
