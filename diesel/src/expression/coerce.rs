use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_builder::*;
use result::QueryResult;

#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
/// Coerces an expression to be another type. No checks are performed to ensure
/// that the new type is valid in all positions that the previous type was.
/// This does not perform an actual cast, it just lies to our type system.
///
/// This is used for a few expressions where we know that the types are actually
/// always interchangeable. (Examples of this include `Timestamp` vs
/// `Timestamptz`, `VarChar` vs `Text`, and `Json` vs `Jsonb`).
///
/// This struct should not be considered a general solution to equivalent types.
/// It is a short term workaround for expressions which are known to be commonly
/// used.
pub struct Coerce<T, ST> {
    expr: T,
    _marker: PhantomData<ST>,
}

impl<T, ST> Coerce<T, ST> {
    pub fn new(expr: T) -> Self {
        Coerce {
            expr: expr,
            _marker: PhantomData,
        }
    }
}

impl<T, ST> Expression for Coerce<T, ST> where
    T: Expression,
{
    type SqlType = ST;
}

impl<T, ST, QS> SelectableExpression<QS> for Coerce<T, ST> where
    T: SelectableExpression<QS>,
{
    type SqlTypeForSelect = Self::SqlType;
}

impl<T, ST, QS> AppearsOnTable<QS> for Coerce<T, ST> where
    T: AppearsOnTable<QS>,
{
}

impl<T, ST, DB> QueryFragment<DB> for Coerce<T, ST> where
    T: QueryFragment<DB>,
    DB: Backend,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.expr.to_sql(out)
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        self.expr.collect_binds(out)
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.expr.is_safe_to_cache_prepared()
    }
}

impl<T: QueryId, ST: 'static> QueryId for Coerce<T, ST> {
    type QueryId = Coerce<T::QueryId, ST>;

    fn has_static_query_id() -> bool {
        true
    }
}

impl<T, ST> NonAggregate for Coerce<T, ST> where
    T: NonAggregate,
    Coerce<T, ST>: Expression,
{
}
