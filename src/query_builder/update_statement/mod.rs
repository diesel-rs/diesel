pub mod changeset;
mod target;

pub use self::changeset::Changeset;
pub use self::target::UpdateTarget;

use expression::Expression;
use query_builder::{QueryFragment, QueryBuilder, BuildQueryResult};

pub fn update<T: UpdateTarget>(source: T) -> IncompleteUpdateStatement<T> {
    IncompleteUpdateStatement(source)
}

pub struct IncompleteUpdateStatement<T>(T);

impl<T> IncompleteUpdateStatement<T> {
    pub fn set<U>(self, values: U) -> UpdateStatement<T, U::Changeset> where
        U: changeset::AsChangeset,
        UpdateStatement<T, U>: QueryFragment,
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
