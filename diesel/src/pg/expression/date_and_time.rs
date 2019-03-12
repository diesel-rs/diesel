use expression::Expression;
use pg::Pg;
use query_builder::*;
use result::QueryResult;
use sql_types::{Date, Timestamp, Timestamptz, VarChar};

/// Marker trait for types which are valid in `AT TIME ZONE` expressions
pub trait DateTimeLike {}
impl DateTimeLike for Date {}
impl DateTimeLike for Timestamp {}
impl DateTimeLike for Timestamptz {}

#[derive(Debug, Copy, Clone, QueryId, NonAggregate)]
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

impl<Ts, Tz> Expression for AtTimeZone<Ts, Tz>
where
    Ts: Expression,
    Ts::SqlType: DateTimeLike,
    Tz: Expression<SqlType = VarChar>,
{
    type SqlType = Timestamp;
}

impl<Ts, Tz> QueryFragment<Pg> for AtTimeZone<Ts, Tz>
where
    Ts: QueryFragment<Pg>,
    Tz: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        self.timestamp.walk_ast(out.reborrow())?;
        out.push_sql(" AT TIME ZONE ");
        self.timezone.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl_selectable_expression!(AtTimeZone<Ts, Tz>);
