use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::QuerySource;

/// Types which can be passed to
/// [`update.set`](struct.IncompleteUpdateStatement.html#method.set). This can
/// be automatically generated for structs by
/// [`#[changeset_for]`](https://github.com/sgrif/diesel/tree/master/diesel_codegen#changeset_fortable_name).
pub trait AsChangeset {
    type Changeset: Changeset;

    fn as_changeset(self) -> Self::Changeset;
}

/// Apps should not need to concern themselves with this trait.
pub trait Changeset {
    type Target: QuerySource;

    fn is_noop(&self) -> bool;
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

impl<T: Changeset + ?Sized> Changeset for Box<T> {
    type Target = T::Target;

    fn is_noop(&self) -> bool {
        (&**self).is_noop()
    }

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        (&**self).to_sql(out)
    }
}

impl<T: Changeset> Changeset for Option<T> {
    type Target = T::Target;

    fn is_noop(&self) -> bool {
        self.is_none()
    }

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        match self {
            &Some(ref c) => c.to_sql(out),
            &None => Ok(()),
        }
    }
}
