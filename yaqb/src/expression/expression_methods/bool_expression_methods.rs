use expression::grouped::Grouped;
use expression::predicates::{And, Or};
use expression::{Expression, AsExpression};
use types::Bool;

pub trait BoolExpressionMethods: Expression<SqlType=Bool> + Sized {
    fn and<T: AsExpression<Bool>>(self, other: T) -> And<Self, T::Expression> {
        And::new(self.as_expression(), other.as_expression())
    }

    fn or<T: AsExpression<Bool>>(self, other: T) -> Grouped<Or<Self, T::Expression>> {
        Grouped(Or::new(self, other.as_expression()))
    }
}

impl<T: Expression<SqlType=Bool>> BoolExpressionMethods for T {}
