use expression::{Expression, AsExpression};
use expression::predicates::{Like, NotLike};
use types::Text;

pub trait TextExpressionMethods: Expression<SqlType=Text> + Sized {
    /// Returns a SQL `LIKE` expression
    fn like<T: AsExpression<Text>>(self, other: T) -> Like<Self, T::Expression> {
        Like::new(self.as_expression(), other.as_expression())
    }

    /// Returns a SQL `NOT LIKE` expression
    fn not_like<T: AsExpression<Text>>(self, other: T) -> NotLike<Self, T::Expression> {
        NotLike::new(self.as_expression(), other.as_expression())
    }
}

impl<T: Expression<SqlType=Text>> TextExpressionMethods for T {}
