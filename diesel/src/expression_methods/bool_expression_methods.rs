use expression::grouped::Grouped;
use expression::operators::{And, Or};
use expression::{Expression, AsExpression};
use types::Bool;

pub trait BoolExpressionMethods: Expression<SqlType=Bool> + Sized {
    /// Creates a SQL `AND` expression
    fn and<T: AsExpression<Bool>>(self, other: T) -> And<Self, T::Expression> {
        And::new(self.as_expression(), other.as_expression())
    }

    /// Creates a SQL `OR` expression
    ///
    /// The result will be wrapped in parenthesis, so that precidence matches
    /// that of your function calls. For example, `false.and(true.or(false))`
    /// will return `false`
    fn or<T: AsExpression<Bool>>(self, other: T) -> Grouped<Or<Self, T::Expression>> {
        Grouped(Or::new(self, other.as_expression()))
    }
}

impl<T: Expression<SqlType=Bool>> BoolExpressionMethods for T {}
