use crate::backend::Backend;
use crate::expression::subselect::Subselect;
use crate::expression::{AppearsOnTable, Expression, SelectableExpression, ValidGrouping};
use crate::helper_types::exists;
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::Bool;

/// Creates a SQL `EXISTS` expression.
///
/// The argument must be a complete SQL query. The query may reference columns
/// from the outer table.
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::select;
/// #     use diesel::dsl::exists;
/// #     let connection = &mut establish_connection();
/// let sean_exists = select(exists(users.filter(name.eq("Sean"))))
///     .get_result(connection);
/// let jim_exists = select(exists(users.filter(name.eq("Jim"))))
///     .get_result(connection);
/// assert_eq!(Ok(true), sean_exists);
/// assert_eq!(Ok(false), jim_exists);
/// # }
/// ```
pub fn exists<T>(query: T) -> exists<T> {
    Exists(Subselect::new(query))
}

#[derive(Clone, Copy, QueryId, Debug)]
pub struct Exists<T>(pub Subselect<T, Bool>);

impl<T> Expression for Exists<T>
where
    Subselect<T, Bool>: Expression,
{
    type SqlType = Bool;
}

impl<T, GB> ValidGrouping<GB> for Exists<T>
where
    Subselect<T, Bool>: ValidGrouping<GB>,
{
    type IsAggregate = <Subselect<T, Bool> as ValidGrouping<GB>>::IsAggregate;
}

#[cfg(not(feature = "unstable"))]
impl<T, DB> QueryFragment<DB> for Exists<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("EXISTS (");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

#[cfg(feature = "unstable")]
impl<T, DB> QueryFragment<DB> for Exists<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    default fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("EXISTS (");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T, QS> SelectableExpression<QS> for Exists<T>
where
    Self: AppearsOnTable<QS>,
    Subselect<T, Bool>: SelectableExpression<QS>,
{
}

impl<T, QS> AppearsOnTable<QS> for Exists<T>
where
    Self: Expression,
    Subselect<T, Bool>: AppearsOnTable<QS>,
{
}
