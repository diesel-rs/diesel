use backend::Backend;
use expression::{Expression, NonAggregate};
use query_builder::*;
use result::QueryResult;
use types::*;

/// Represents the SQL `CURRENT_TIMESTAMP` constant. This is equivalent to the
/// `NOW()` function on backends that support it.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub struct now;

impl Expression for now {
    type SqlType = Timestamp;
}

impl NonAggregate for now {
}

impl<DB: Backend> QueryFragment<DB> for now {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("CURRENT_TIMESTAMP");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl_query_id!(now);
impl_selectable_expression!(now);

operator_allowed!(now, Add, add);
operator_allowed!(now, Sub, sub);
sql_function!(date, date_t, (x: Timestamp) -> Date,
"Represents the SQL `DATE` function. The argument should be a Timestamp
expression, and the return value will be an expression of type Date");

#[cfg(feature="postgres")]
use expression::AsExpression;
#[cfg(feature="postgres")]
use expression::coerce::Coerce;
#[cfg(feature="postgres")]
use types::Timestamptz;

#[cfg(feature="postgres")]
impl AsExpression<Timestamptz> for now {
    type Expression = Coerce<now, Timestamptz>;

    fn as_expression(self) -> Self::Expression {
        Coerce::new(self)
    }
}
