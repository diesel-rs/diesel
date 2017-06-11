use expression::*;
use query_builder::*;
use result::QueryResult;
use types::Bool;

/// Creates a SQL `NOT` expression
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// use diesel::expression::not;
///
/// let users_with_name = users.select(id).filter(name.eq("Sean"));
/// let users_not_with_name = users.select(id).filter(
///     not(name.eq("Sean")));
///
/// assert_eq!(Ok(1), users_with_name.first(&connection));
/// assert_eq!(Ok(2), users_not_with_name.first(&connection));
/// # }
/// ```
pub fn not<T: AsExpression<Bool>>(expr: T) -> Not<T::Expression> {
    Not(expr.as_expression())
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct Not<T>(T);

impl<T: Expression<SqlType=Bool>> Expression for Not<T> {
    type SqlType = Bool;
}

impl<T, QS> AppearsOnTable<QS> for Not<T> where
    T: AppearsOnTable<QS>,
    Not<T>: Expression,
{
}

impl<T, QS> SelectableExpression<QS> for Not<T> where
    T: SelectableExpression<QS>,
    Not<T>: AppearsOnTable<QS>,
{
}

impl<T: NonAggregate> NonAggregate for Not<T> {}

impl<T, DB> QueryFragment<DB> for Not<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("NOT (");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_query_id!(Not<T>);
