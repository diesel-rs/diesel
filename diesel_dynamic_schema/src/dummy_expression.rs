use diesel::expression::{AppearsOnTable, Expression, NonAggregate, SelectableExpression};

#[doc(hidden)]
/// A dummy expression.
pub struct DummyExpression;

impl DummyExpression {
    pub(crate) fn new() -> Self {
        DummyExpression
    }
}

impl<QS> SelectableExpression<QS> for DummyExpression {}

impl<QS> AppearsOnTable<QS> for DummyExpression {}

impl Expression for DummyExpression {
    type SqlType = ();
}

impl NonAggregate for DummyExpression {}
