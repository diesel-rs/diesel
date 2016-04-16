use backend::Backend;
use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::*;
use result::QueryResult;
use types::*;

/// Represents the SQL CURRENT_TIMESTAMP constant. This is equivalent to the
/// `NOW()` function on backends that support it.
#[allow(non_camel_case_types)]
pub struct now;

impl Expression for now {
    type SqlType = Timestamp;
}

impl<QS> SelectableExpression<QS> for now {
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
}

operator_allowed!(now, Add, add);
operator_allowed!(now, Sub, sub);
sql_function!(date, date_t, (x: Timestamp) -> Date,
"Represents the SQL DATE() function. The argument should be a Timestamp
expression, and the return value will be an expression of type Date");
