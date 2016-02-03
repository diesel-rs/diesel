use backend::*;
use expression::{Expression, SelectableExpression, NonAggregate};
use pg::{Pg, PgQueryBuilder};
use query_builder::*;
use types::{Timestamp, VarChar};

pub struct AtTimeZone<Ts, Tz> {
    timestamp: Ts,
    timezone: Tz,
}

impl<Ts, Tz> AtTimeZone<Ts, Tz> {
    pub fn new(timestamp: Ts, timezone: Tz) -> Self {
        AtTimeZone {
            timestamp: timestamp,
            timezone: timezone,
        }
    }
}

impl<Ts, Tz> Expression for AtTimeZone<Ts, Tz> where
    Ts: Expression<SqlType=Timestamp>,
    Tz: Expression<SqlType=VarChar>,
{
    // FIXME: This should be Timestamptz when we support that type
    type SqlType = Timestamp;
}

impl<Ts, Tz> NonAggregate for AtTimeZone<Ts, Tz> where
    AtTimeZone<Ts, Tz>: Expression,
{
}

impl<Ts, Tz> QueryFragment<Pg> for AtTimeZone<Ts, Tz> where
    Ts: QueryFragment<Pg>,
    Tz: QueryFragment<Pg>,
{
    fn to_sql(&self, out: &mut PgQueryBuilder) -> BuildQueryResult {
        try!(self.timestamp.to_sql(out));
        out.push_sql(" AT TIME ZONE ");
        self.timezone.to_sql(out)
    }
}

impl<Ts, Tz> QueryFragment<Debug> for AtTimeZone<Ts, Tz> where
    Ts: QueryFragment<Debug>,
    Tz: QueryFragment<Debug>,
{
    fn to_sql(&self, out: &mut <Debug as Backend>::QueryBuilder) -> BuildQueryResult {
        try!(self.timestamp.to_sql(out));
        out.push_sql(" AT TIME ZONE ");
        self.timezone.to_sql(out)
    }
}

impl<Ts, Tz, Qs> SelectableExpression<Qs> for AtTimeZone<Ts, Tz> where
    AtTimeZone<Ts, Tz>: Expression,
    Ts: SelectableExpression<Qs>,
    Tz: SelectableExpression<Tz>,
{}
