use backend::Backend;
use query_builder::BuildQueryResult;
use query_source::QuerySource;
use result::QueryResult;

/// Types which can be passed to
/// [`update.set`](struct.IncompleteUpdateStatement.html#method.set). This can
/// be automatically generated for structs by
/// [`#[changeset_for]`](https://github.com/sgrif/diesel/tree/master/diesel_codegen#changeset_fortable_name).
pub trait AsChangeset {
    type Target: QuerySource;
    type Changeset;

    fn as_changeset(self) -> Self::Changeset;
}

/// Apps should not need to concern themselves with this trait.
pub trait Changeset<DB: Backend> {
    fn is_noop(&self) -> bool;
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()>;
}

impl<T: AsChangeset> AsChangeset for Option<T> {
    type Target = T::Target;
    type Changeset = Option<T::Changeset>;

    fn as_changeset(self) -> Self::Changeset {
        self.map(|v| v.as_changeset())
    }
}

impl<T: Changeset<DB>, DB: Backend> Changeset<DB> for Option<T> {
    fn is_noop(&self) -> bool {
        self.is_none()
    }

    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        match self {
            &Some(ref c) => c.to_sql(out),
            &None => Ok(()),
        }
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        match self {
            &Some(ref c) => c.collect_binds(out),
            &None => Ok(()),
        }
    }
}
