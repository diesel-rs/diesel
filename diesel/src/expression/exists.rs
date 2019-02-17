use backend::Backend;
use expression::subselect::Subselect;
use expression::{AppearsOnTable, Expression, NonAggregate, SelectableExpression};
use query_builder::*;
use result::QueryResult;
use sql_types::Bool;

/// Creates a SQL `EXISTS` expression.
///
/// The argument must be a complete SQL query. The query may reference columns
/// from the outer table.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::select;
/// #     use diesel::dsl::exists;
/// #     let connection = establish_connection();
/// let sean_exists = select(exists(users.filter(name.eq("Sean"))))
///     .get_result(&connection);
/// let jim_exists = select(exists(users.filter(name.eq("Jim"))))
///     .get_result(&connection);
/// assert_eq!(Ok(true), sean_exists);
/// assert_eq!(Ok(false), jim_exists);
/// # }
/// ```
pub fn exists<T>(query: T) -> Exists<T> {
    Exists(Subselect::new(query))
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Exists<T>(Subselect<T, ()>);

impl<T> Expression for Exists<T>
where
    Subselect<T, ()>: Expression,
{
    type SqlType = Bool;
}

impl<T> NonAggregate for Exists<T> where Subselect<T, ()>: NonAggregate {}

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

impl<T, QS> SelectableExpression<QS> for Exists<T>
where
    Self: AppearsOnTable<QS>,
    Subselect<T, ()>: SelectableExpression<QS>,
{
}

impl<T, QS> AppearsOnTable<QS> for Exists<T>
where
    Self: Expression,
    Subselect<T, ()>: AppearsOnTable<QS>,
{
}
