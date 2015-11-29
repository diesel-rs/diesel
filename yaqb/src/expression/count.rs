use query_builder::{QueryBuilder, BuildQueryResult};
use super::{Expression, SelectableExpression};
use types::BigInt;

/// Creates a SQL `COUNT` expression
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `yaqb::expression::count`, or glob import
/// `yaqb::expression::dsl::*`
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
/// it specifically as `yaqb::expression::count_star`, or glob import
/// `yaqb::expression::dsl::*`
///
/// # Example
///
/// # #[macro_use] extern crate yaqb;
/// # include!("src/doctest_setup.rs");
/// # use yaqb::expression::dsl::*;
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
/// assert_eq!(Some(2), users.select(count_star()).first(&connection).unwrap());
/// # }
pub fn count_star() -> CountStar {
    CountStar
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct Count<T: Expression> {
    target: T,
}

impl<T: Expression> Expression for Count<T> {
    type SqlType = BigInt;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("COUNT(");
        try!(self.target.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

impl<T: Expression, QS> SelectableExpression<QS> for Count<T> {
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct CountStar;

impl Expression for CountStar {
    type SqlType = BigInt;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("COUNT(*)");
        Ok(())
    }
}

impl<QS> SelectableExpression<QS> for CountStar {
}
