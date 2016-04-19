use backend::Backend;
use query_builder::*;
use result::QueryResult;
use super::{Expression, SelectableExpression};
use types::BigInt;

/// Creates a SQL `COUNT` expression
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::expression::count`, or glob import
/// `diesel::expression::dsl::*`
pub fn count<T: Expression>(t: T) -> Count<T> {
    Count {
        target: t,
    }
}

/// Creates a SQL `COUNT(*)` expression
///
/// For selecting the count of a query, and nothing else, you can just call
/// [`count`](../../trait.CountDsl.html) on the query instead.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::expression::count_star`, or glob import
/// `diesel::expression::dsl::*`
///
/// # Example
///
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// # use diesel::expression::dsl::*;
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// assert_eq!(Ok(Some(2)), users.select(count_star()).first(&connection));
/// # }
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
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("COUNT(");
        try!(self.target.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.target.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.target.is_safe_to_cache_prepared()
    }
}

impl_query_id!(Count<T>);

impl<T: Expression, QS> SelectableExpression<QS> for Count<T> {
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct CountStar;

impl Expression for CountStar {
    type SqlType = BigInt;
}

impl<DB: Backend> QueryFragment<DB> for CountStar {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("COUNT(*)");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl<QS> SelectableExpression<QS> for CountStar {
}

impl_query_id!(CountStar);
