use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_builder::*;
use result::QueryResult;

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
{
    type SqlType = ST;
}

impl<T, ST, QS> SelectableExpression<QS> for Coerce<T, ST> where T: SelectableExpression<QS> {}

impl<T, ST, QS> AppearsOnTable<QS> for Coerce<T, ST> where T: AppearsOnTable<QS> {}

impl<T, ST, DB> QueryFragment<DB> for Coerce<T, ST>
where
    T: QueryFragment<DB>,
    DB: Backend,
{
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        self.expr.walk_ast(pass)
    }
}

impl<T, ST> NonAggregate for Coerce<T, ST> where T: NonAggregate {}
