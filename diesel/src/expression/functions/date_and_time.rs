use crate::backend::Backend;
use crate::expression::coerce::Coerce;
use crate::expression::functions::sql_function;
use crate::expression::{AsExpression, Expression, ValidGrouping};
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::*;

/// Represents the SQL `CURRENT_TIMESTAMP` constant. This is equivalent to the
/// `NOW()` function on backends that support it.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
pub struct now;

impl Expression for now {
    type SqlType = Timestamp;
}

impl<DB: Backend> QueryFragment<DB> for now {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("CURRENT_TIMESTAMP");
        Ok(())
    }
}

impl_selectable_expression!(now);

operator_allowed!(now, Add, add);
operator_allowed!(now, Sub, sub);
sql_function! {
    /// Represents the SQL `DATE` function. The argument should be a Timestamp
    /// expression, and the return value will be an expression of type Date.

    /// # Examples

    /// ```ignore
    /// # extern crate chrono;
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     let connection = establish_connection();
    /// let today: chrono::NaiveDate = diesel::select(date(now)).first(&connection).unwrap();
    /// # }
    /// ```
    fn date(expr: Timestamp) -> Date;
}

impl AsExpression<Nullable<Timestamp>> for now {
    type Expression = Coerce<now, Nullable<Timestamp>>;

    fn as_expression(self) -> Self::Expression {
        Coerce::new(self)
    }
}

#[cfg(feature = "postgres")]
impl AsExpression<Timestamptz> for now {
    type Expression = Coerce<now, Timestamptz>;

    fn as_expression(self) -> Self::Expression {
        Coerce::new(self)
    }
}

#[cfg(feature = "postgres")]
impl AsExpression<Nullable<Timestamptz>> for now {
    type Expression = Coerce<now, Nullable<Timestamptz>>;

    fn as_expression(self) -> Self::Expression {
        Coerce::new(self)
    }
}

/// Represents the SQL `CURRENT_DATE` constant.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
pub struct today;

impl Expression for today {
    type SqlType = Date;
}

impl<DB: Backend> QueryFragment<DB> for today {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("CURRENT_DATE");
        Ok(())
    }
}

impl_selectable_expression!(today);

operator_allowed!(today, Add, add);
operator_allowed!(today, Sub, sub);

impl AsExpression<Nullable<Date>> for today {
    type Expression = Coerce<today, Nullable<Date>>;

    fn as_expression(self) -> Self::Expression {
        Coerce::new(self)
    }
}
