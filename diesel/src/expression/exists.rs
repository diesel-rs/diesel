use backend::Backend;
use expression::{Expression, NonAggregate};
use query_builder::*;
use result::QueryResult;
use types::Bool;

/// Creates a SQL `EXISTS` expression.
///
/// The argument must be a complete SQL query. The result of this could in
/// theory be passed to `.filter`, but since the query cannot reference columns
/// from the outer query, this is of limited usefulness.
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
/// #     use diesel::select;
/// #     use diesel::expression::dsl::exists;
/// #     let connection = establish_connection();
/// let sean_exists = select(exists(users.filter(name.eq("Sean"))))
///     .get_result(&connection);
/// let jim_exists = select(exists(users.filter(name.eq("Jim"))))
///     .get_result(&connection);
/// assert_eq!(Ok(true), sean_exists);
/// assert_eq!(Ok(false), jim_exists);
/// # }
/// ```
pub fn exists<T: AsQuery>(query: T) -> Exists<T::Query> {
    Exists(query.as_query())
}

#[derive(Debug, Clone, Copy)]
pub struct Exists<T>(T);

impl<T> Expression for Exists<T> where
    T: Query,
{
    type SqlType = Bool;
}

impl<T> NonAggregate for Exists<T> {
}

impl<T, DB> QueryFragment<DB> for Exists<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("EXISTS (");
        try!(self.0.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.0.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.0.is_safe_to_cache_prepared()
    }
}

impl_query_id!(Exists<T>);
impl_selectable_expression!(Exists<T>);
