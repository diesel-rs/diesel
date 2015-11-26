use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::QuerySource;

pub trait AsChangeset {
    type Changeset: Changeset;

    fn as_changeset(self) -> Self::Changeset;
}

pub trait Changeset {
    type Target: QuerySource;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult;
}

impl<T> AsChangeset for T where
    T: Changeset,
{
    type Changeset = Self;

    fn as_changeset(self) -> Self::Changeset {
        self
    }
}
