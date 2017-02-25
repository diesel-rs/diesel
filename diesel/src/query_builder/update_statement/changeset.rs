use backend::Backend;
use hlist::*;
use query_builder::BuildQueryResult;
use query_source::QuerySource;
use result::QueryResult;

/// Types which can be passed to
/// [`update.set`](/diesel/query_builder/struct.IncompleteUpdateStatement.html#method.set).
///
/// ### Deriving
///
/// This trait can be automatically derived using `diesel_codegen` by adding
/// `#[derive(AsChangeset)]` to your struct.  Structs which derive this trait
/// must be annotated with `#[table_name = "something"]`. If the field name of
/// your struct differs from the name of the column, you can annotate the field
/// with `#[column_name = "some_column_name"]`.
///
/// By default, any `Option` fields on the struct are skipped if their value is
/// `None`. If you would like to assign `NULL` to the field instead, you can
/// annotate your struct with `#[changeset_options(treat_none_as_null =
/// "true")]`.
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
        match *self {
            Some(ref c) => c.to_sql(out),
            None => Ok(()),
        }
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        match *self {
            Some(ref c) => c.collect_binds(out),
            None => Ok(()),
        }
    }
}

impl<Head, Head2, Tail> AsChangeset for Cons<Head, Cons<Head2, Tail>> where
    Head: AsChangeset,
    Cons<Head2, Tail>: AsChangeset<Target=Head::Target>,
{
    type Target = Head::Target;
    type Changeset = Cons<
        Head::Changeset,
        <Cons<Head2, Tail> as AsChangeset>::Changeset,
    >;

    fn as_changeset(self) -> Self::Changeset {
        Cons(
            self.0.as_changeset(),
            self.1.as_changeset(),
        )
    }
}

impl<Head> AsChangeset for Cons<Head, Nil> where
    Head: AsChangeset,
{
    type Target = Head::Target;
    type Changeset = Cons<Head::Changeset, Nil>;

    fn as_changeset(self) -> Self::Changeset {
        Cons(
            self.0.as_changeset(),
            Nil,
        )
    }
}

impl<Head, Tail, DB> Changeset<DB> for Cons<Head, Tail> where
    DB: Backend,
    Head: Changeset<DB>,
    Tail: Changeset<DB>,
{
    fn is_noop(&self) -> bool {
        self.0.is_noop() && self.1.is_noop()
    }

    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        use query_builder::QueryBuilder;

        if !self.0.is_noop() {
            try!(self.0.to_sql(out));
            if !self.1.is_noop() {
                out.push_sql(", ");
            }
        }
        self.1.to_sql(out)
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.0.collect_binds(out));
        self.1.collect_binds(out)
    }
}

impl<DB: Backend> Changeset<DB> for Nil {
    fn is_noop(&self) -> bool {
        true
    }

    fn to_sql(&self, _: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }

    fn collect_binds(&self, _: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }
}
