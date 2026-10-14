use crate::backend::Backend;
use crate::expression::coerce::Coerce;
use crate::expression::functions::declare_sql_function;
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
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("CURRENT_TIMESTAMP");
        Ok(())
    }
}

impl_selectable_expression!(now);

operator_allowed!(now, Add, add);
operator_allowed!(now, Sub, sub);

#[declare_sql_function]
extern "SQL" {
    /// Represents the SQL `DATE` function. The argument should be a Timestamp
    /// expression, and the return value will be an expression of type Date.
    ///
    /// # Examples
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::{now, date};
    /// # use diesel::deserialize::Queryable;
    /// #
    /// # fn test<R: Queryable<diesel::sql_types::Date, DB> + 'static>() -> QueryResult<R> {
    /// #     let connection = &mut establish_connection();
    /// let today = diesel::select(date(now)).first(connection)?;
    /// #     Ok(today)
    /// # }
    /// # fn main() {
    /// #
    /// # }
    /// ```
    fn date(expr: Timestamp) -> Date;
}

/// Implements `AsExpression<$sql_type>` for `$expr`, `&$expr` and `&&$expr`, each
/// coercing `$expr` to `$sql_type`.
///
/// The reference impls mirror the `&T` and `&&T` impls that `#[derive(AsExpression)]`
/// generates: `#[derive(AsChangeset)]` on a struct also implements `AsChangeset` for a
/// reference to it, built from references to its fields. Passing a struct by reference
/// to `.set()`, as batch updates do, therefore needs `&now` for a `now` field and
/// `&&now` for a `&now` field.
macro_rules! coerce_as_expression {
    ($expr:ident => $sql_type:ty) => {
        impl AsExpression<$sql_type> for $expr {
            type Expression = Coerce<$expr, $sql_type>;

            fn as_expression(self) -> Self::Expression {
                Coerce::new(self)
            }
        }

        impl AsExpression<$sql_type> for &$expr {
            type Expression = Coerce<$expr, $sql_type>;

            fn as_expression(self) -> Self::Expression {
                Coerce::new(*self)
            }
        }

        impl AsExpression<$sql_type> for &&$expr {
            type Expression = Coerce<$expr, $sql_type>;

            fn as_expression(self) -> Self::Expression {
                Coerce::new(**self)
            }
        }
    };
}

coerce_as_expression!(now => Nullable<Timestamp>);

#[cfg(feature = "postgres_backend")]
coerce_as_expression!(now => Timestamptz);

#[cfg(feature = "postgres_backend")]
coerce_as_expression!(now => Nullable<Timestamptz>);

#[cfg(feature = "__sqlite-shared")]
coerce_as_expression!(now => TimestamptzSqlite);

#[cfg(feature = "__sqlite-shared")]
coerce_as_expression!(now => Nullable<TimestamptzSqlite>);

/// Represents the SQL `CURRENT_DATE` constant.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
pub struct today;

impl Expression for today {
    type SqlType = Date;
}

impl<DB: Backend> QueryFragment<DB> for today {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("CURRENT_DATE");
        Ok(())
    }
}

impl_selectable_expression!(today);

operator_allowed!(today, Add, add);
operator_allowed!(today, Sub, sub);

coerce_as_expression!(today => Nullable<Date>);
