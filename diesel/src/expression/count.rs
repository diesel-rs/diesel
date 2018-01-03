use backend::Backend;
use query_builder::*;
use result::QueryResult;
use super::Expression;
use types::BigInt;

/// Creates a SQL `COUNT` expression
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::dsl::count`, or glob import
/// `diesel::dsl::*`
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::animals::dsl::*;
/// #     let connection = establish_connection();
/// assert_eq!(Ok(1), animals.select(count(name)).first(&connection));
/// # }
/// ```
pub fn count<T: Expression>(t: T) -> Count<T> {
    Count { target: t }
}

/// Creates a SQL `COUNT(*)` expression
///
/// For selecting the count of a query, and nothing else, you can just call
/// [`count`](../query_dsl/trait.QueryDsl.html#method.count)
/// on the query instead.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::dsl::count_star`, or glob import
/// `diesel::dsl::*`
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// assert_eq!(Ok(2), users.select(count_star()).first(&connection));
/// # }
/// ```
pub fn count_star() -> CountStar {
    CountStar
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct Count<T> {
    target: T,
}

impl<T: Expression> Expression for Count<T> {
    type SqlType = BigInt;
}

impl<T: QueryFragment<DB>, DB: Backend> QueryFragment<DB> for Count<T> {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("COUNT(");
        self.target.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_query_id!(Count<T>);
impl_selectable_expression!(Count<T>);

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct CountStar;

impl Expression for CountStar {
    type SqlType = BigInt;
}

impl<DB: Backend> QueryFragment<DB> for CountStar {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("COUNT(*)");
        Ok(())
    }
}

impl_query_id!(CountStar);
impl_selectable_expression!(CountStar);
