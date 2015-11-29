#[macro_export]
macro_rules! infix_predicate {
    ($name:ident, $operator:expr) => {
        infix_predicate!($name, $operator, $crate::types::Bool);
    };

    ($name:ident, $operator:expr, $return_type:ty) => {
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

        impl<T, U> $crate::expression::Expression for $name<T, U> where
            T: $crate::expression::Expression,
            U: $crate::expression::Expression,
        {
            type SqlType = $return_type;

            fn to_sql(&self, out: &mut $crate::query_builder::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!(self.left.to_sql(out));
                out.push_sql($operator);
                self.right.to_sql(out)
            }
        }

        impl<T, U, QS> $crate::expression::SelectableExpression<QS> for $name<T, U> where
            T: $crate::expression::SelectableExpression<QS>,
            U: $crate::expression::SelectableExpression<QS>,
        {
        }

        impl<T, U> $crate::expression::NonAggregate for $name<T, U> where
            T: $crate::expression::NonAggregate,
            U: $crate::expression::NonAggregate,
        {
        }
    }
}

#[macro_export]
macro_rules! postfix_predicate {
    ($name:ident, $operator:expr) => {
        postfix_predicate!($name, $operator, $crate::types::Bool);
    };

    ($name:ident, $operator:expr, $return_type:ty) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<T> {
            expr: T,
        }

        impl<T> $name<T> {
            pub fn new(expr: T) -> Self {
                $name {
                    expr: expr,
                }
            }
        }

        impl<T> $crate::expression::Expression for $name<T> where
            T: $crate::expression::Expression,
        {
            type SqlType = $return_type;

            fn to_sql(&self, out: &mut $crate::query_builder::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!(self.expr.to_sql(out));
                out.push_sql($operator);
                Ok(())
            }
        }

        impl<T, QS> $crate::expression::SelectableExpression<QS> for $name<T> where
            T: $crate::expression::SelectableExpression<QS>,
        {
        }

        impl<T> $crate::expression::NonAggregate for $name<T> where
            T: $crate::expression::NonAggregate,
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

postfix_predicate!(IsNull, " IS NULL");
postfix_predicate!(IsNotNull, " IS NOT NULL");

use query_source::Column;
use query_builder::*;
use super::{Expression, SelectableExpression};

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
