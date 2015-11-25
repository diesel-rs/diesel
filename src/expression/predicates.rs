use query_builder::*;
use super::{Expression, SelectableExpression, NonAggregate};
use types::Bool;

macro_rules! infix_predicate {
    ($name:ident, $operator:expr) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<T, U> {
            left: T,
            right: U,
        }

        impl<T, U> $name<T, U> {
            pub fn new(left: T, right: U) -> Self {
                $name {
                    left: left,
                    right: right,
                }
            }
        }

        impl<T, U> Expression for $name<T, U> where
            T: Expression,
            U: Expression,
        {
            type SqlType = Bool;

            fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                try!(self.left.to_sql(out));
                out.push_sql($operator);
                self.right.to_sql(out)
            }
        }

        impl<T, U, QS> SelectableExpression<QS> for $name<T, U> where
            T: SelectableExpression<QS>,
            U: SelectableExpression<QS>,
        {
        }

        impl<T, U> NonAggregate for $name<T, U> where
            T: NonAggregate,
            U: NonAggregate,
        {
        }
    }
}

infix_predicate!(And, " AND ");
infix_predicate!(Between, " BETWEEN ");
infix_predicate!(Eq, " = ");
infix_predicate!(Gt, " > ");
infix_predicate!(GtEq, " >= ");
infix_predicate!(Like, " LIKE ");
infix_predicate!(Lt, " < ");
infix_predicate!(LtEq, " <= ");
infix_predicate!(NotBetween, " NOT BETWEEN ");
infix_predicate!(NotEq, " != ");
infix_predicate!(NotLike, " NOT LIKE ");
infix_predicate!(Or, " OR ");

use query_source::Column;

impl<T, U> Changeset for Eq<T, U> where
    T: Column,
    U: SelectableExpression<T::Table>,
    Eq<T, U>: Expression,
{
    type Target = T::Table;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(out.push_identifier(T::name()));
        out.push_sql(" = ");
        Expression::to_sql(&self.right, out)
    }
}
