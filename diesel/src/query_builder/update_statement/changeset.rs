use backend::Backend;
use query_builder::AstPass;
use query_source::QuerySource;
use result::QueryResult;

/// Types which can be passed to
/// [`update.set`](/diesel/query_builder/struct.IncompleteUpdateStatement.html#method.set).
///
/// ### Deriving
///
/// This trait can be automatically derived using by adding `#[derive(AsChangeset)]`
/// to your struct.  Structs which derive this trait must be annotated with
/// `#[table_name = "something"]`. If the field name of your struct differs
/// from the name of the column, you can annotate the field with
/// `#[column_name = "some_column_name"]`.
///
/// By default, any `Option` fields on the struct are skipped if their value is
/// `None`. If you would like to assign `NULL` to the field instead, you can
/// annotate your struct with `#[changeset_options(treat_none_as_null =
/// "true")]`.
pub trait AsChangeset {
    /// The table which `Self::Changeset` will be updating
    type Target: QuerySource;

    /// The update statement this type represents
    type Changeset;

    /// Convert `self` into the actual update statement being executed
    fn as_changeset(self) -> Self::Changeset;
}

#[doc(hidden)]
pub trait Changeset<DB: Backend> {
    /// Does this changeset actually include any changes?
    fn is_noop(&self) -> bool;

    /// See [`QueryFragment#walk_ast`]
    ///
    /// [`QueryFragment#walk_ast`]: trait.QueryFragment.html#tymethod.walk_ast
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()>;
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

    fn walk_ast(&self, out: AstPass<DB>) -> QueryResult<()> {
        match *self {
            Some(ref c) => c.walk_ast(out),
            None => Ok(()),
        }
    }
}
