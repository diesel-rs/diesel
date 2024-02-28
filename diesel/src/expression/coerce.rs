use crate::expression::*;
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::DieselNumericOps;
use std::marker::PhantomData;

#[derive(Debug, Copy, Clone, QueryId, DieselNumericOps)]
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

impl<T, ST> Expression for Coerce<T, ST>
where
    T: Expression,
    ST: SqlType + TypedExpressionType,
{
    type SqlType = ST;
}

impl<T, ST, QS> SelectableExpression<QS> for Coerce<T, ST>
where
    T: SelectableExpression<QS>,
    Self: Expression,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for Coerce<T, ST>
where
    T: AppearsOnTable<QS>,
    Self: Expression,
{
}

impl<T, ST, DB> QueryFragment<DB> for Coerce<T, ST>
where
    T: QueryFragment<DB>,
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.expr.walk_ast(pass)
    }
}

impl<T, ST, GB> ValidGrouping<GB> for Coerce<T, ST>
where
    T: ValidGrouping<GB>,
{
    type IsAggregate = T::IsAggregate;
}
