use backend::Backend;
use expression::operators::Eq;
use expression::AppearsOnTable;
use query_builder::*;
use query_source::{Column, QuerySource};
use result::QueryResult;

/// Types which can be passed to
/// [`update.set`](struct.UpdateStatement.html#method.set).
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

impl<T: AsChangeset> AsChangeset for Option<T> {
    type Target = T::Target;
    type Changeset = Option<T::Changeset>;

    fn as_changeset(self) -> Self::Changeset {
        self.map(|v| v.as_changeset())
    }
}

impl<Left, Right> AsChangeset for Eq<Left, Right>
where
    Left: Column,
    Right: AppearsOnTable<Left::Table>,
{
    type Target = Left::Table;
    type Changeset = Assign<Left, Right>;

    fn as_changeset(self) -> Self::Changeset {
        Assign {
            _column: self.left,
            expr: self.right,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Assign<Col, Expr> {
    _column: Col,
    expr: Expr,
}

impl<T, U, DB> QueryFragment<DB> for Assign<T, U>
where
    DB: Backend,
    T: Column,
    U: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(T::NAME)?;
        out.push_sql(" = ");
        QueryFragment::walk_ast(&self.expr, out)
    }
}
