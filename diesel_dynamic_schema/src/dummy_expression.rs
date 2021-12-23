use diesel::expression::{
    expression_types, is_aggregate, AppearsOnTable, Expression, SelectableExpression, ValidGrouping,
};

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
    type SqlType = expression_types::NotSelectable;
}

impl ValidGrouping<()> for DummyExpression {
    type IsAggregate = is_aggregate::No;
}
