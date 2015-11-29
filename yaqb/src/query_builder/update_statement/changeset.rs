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

impl<T: Changeset> Changeset for Vec<T> {
    type Target = T::Target;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        for (i, changeset) in self.iter().enumerate() {
            if i != 0 {
                out.push_sql(", ");
            }
            try!(changeset.to_sql(out))
        }
        Ok(())
    }
}

impl<T: Changeset> Changeset for Box<T> {
    type Target = T::Target;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        (&**self).to_sql(out)
    }
}
