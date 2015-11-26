use expression::{Expression, AsExpression};
use expression::predicates::{Like, NotLike};
use types::{VarChar, Text};

pub trait VarCharExpressionMethods: Expression<SqlType=VarChar> + Sized {
    fn like<T: AsExpression<VarChar>>(self, other: T) -> Like<Self, T::Expression> {
        Like::new(self.as_expression(), other.as_expression())
    }

    fn not_like<T: AsExpression<VarChar>>(self, other: T) -> NotLike<Self, T::Expression> {
        NotLike::new(self.as_expression(), other.as_expression())
    }
}

impl<T: Expression<SqlType=VarChar>> VarCharExpressionMethods for T {}

pub trait TextExpressionMethods: Expression<SqlType=Text> + Sized {
    fn like<T: AsExpression<Text>>(self, other: T) -> Like<Self, T::Expression> {
        Like::new(self.as_expression(), other.as_expression())
    }

    fn not_like<T: AsExpression<Text>>(self, other: T) -> NotLike<Self, T::Expression> {
        NotLike::new(self.as_expression(), other.as_expression())
    }
}

impl<T: Expression<SqlType=Text>> TextExpressionMethods for T {}
