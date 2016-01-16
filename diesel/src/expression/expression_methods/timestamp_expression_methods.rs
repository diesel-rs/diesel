use expression::{AsExpression, Expression};
use expression::date_and_time::AtTimeZone;
use types::{VarChar, Timestamp};

pub trait TimestampExpressionMethods: Expression<SqlType=Timestamp> + Sized {
    /// Returns a PostgreSQL "AT TIME ZONE" expression
    fn at_time_zone<T>(self, timezone: T) -> AtTimeZone<Self, T::Expression> where
        T: AsExpression<VarChar>,
    {
        AtTimeZone::new(self, timezone.as_expression())
    }
}

impl<T: Expression<SqlType=Timestamp>> TimestampExpressionMethods for T {}
