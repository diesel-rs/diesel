use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::QuerySource;

pub trait AsChangeset {
    type Changeset: Changeset;

    fn as_changeset(self) -> Self::Changeset;
}

pub trait Changeset {
    type Target: QuerySource;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult;
}

impl<T> AsChangeset for T where
    T: Changeset,
{
    type Changeset = Self;

    fn as_changeset(self) -> Self::Changeset {
        self
    }
}
