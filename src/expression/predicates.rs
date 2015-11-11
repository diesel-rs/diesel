use query_builder::*;
use super::{Expression, SelectableExpression};
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

            fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
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
    }
}

infix_predicate!(And, " AND ");
infix_predicate!(Eq, " = ");
