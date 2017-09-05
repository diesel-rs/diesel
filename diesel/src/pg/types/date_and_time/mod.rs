use std::error::Error;
use std::io::Write;
use std::ops::Add;

use pg::Pg;
use types::{self, Date, FromSql, Interval, IsNull, Time, Timestamp, Timestamptz, ToSql,
            ToSqlOutput};

primitive_impls!(Date -> (pg: (1082, 1182)));
primitive_impls!(Time -> (pg: (1083, 1183)));
primitive_impls!(Timestamp -> (pg: (1114, 1115)));
primitive_impls!(Timestamptz -> (pg: (1184, 1185)));
primitive_impls!(Timestamptz);

#[cfg(feature = "quickcheck")]
mod quickcheck_impls;
mod std_time;
#[cfg(feature = "chrono")]
mod chrono;
#[cfg(feature = "deprecated-time")]
mod deprecated_time;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Timestamps are represented in Postgres as a 64 bit signed integer representing the number of
/// microseconds since January 1st 2000. This struct is a dumb wrapper type, meant only to indicate
/// the integer's meaning.
pub struct PgTimestamp(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Dates are represented in Postgres as a 32 bit signed integer representing the number of julian
/// days since January 1st 2000. This struct is a dumb wrapper type, meant only to indicate the
/// integer's meaning.
pub struct PgDate(pub i32);

/// Time is represented in Postgres as a 64 bit signed integer representing the number of
/// microseconds since midnight. This struct is a dumb wrapper type, meant only to indicate the
/// integer's meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PgTime(pub i64);

/// Intervals in Postgres are separated into 3 parts. A 64 bit integer representing time in
/// microseconds, a 32 bit integer representing number of days, and a 32 bit integer
/// representing number of months. This struct is a dumb wrapper type, meant only to indicate the
/// meaning of these parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgInterval {
    pub microseconds: i64,
    pub days: i32,
    pub months: i32,
}

impl PgInterval {
    pub fn new(microseconds: i64, days: i32, months: i32) -> Self {
        PgInterval {
            microseconds: microseconds,
            days: days,
            months: months,
        }
    }

    pub fn from_microseconds(microseconds: i64) -> Self {
        Self::new(microseconds, 0, 0)
    }

    pub fn from_days(days: i32) -> Self {
        Self::new(0, days, 0)
    }

    pub fn from_months(months: i32) -> Self {
        Self::new(0, 0, months)
    }
}

queryable_impls!(Date -> PgDate);
queryable_impls!(Time -> PgTime);
queryable_impls!(Timestamp -> PgTimestamp);
queryable_impls!(Timestamptz -> PgTimestamp);
expression_impls!(Date -> PgDate);
expression_impls!(Time -> PgTime);
expression_impls!(Timestamp -> PgTimestamp);
expression_impls!(Timestamptz -> PgTimestamp);

primitive_impls!(Interval -> (PgInterval, pg: (1186, 1187)));

impl ToSql<types::Timestamp, Pg> for PgTimestamp {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::BigInt, Pg>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Timestamp, Pg> for PgTimestamp {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::BigInt, Pg>::from_sql(bytes).map(PgTimestamp)
    }
}

impl ToSql<types::Timestamptz, Pg> for PgTimestamp {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::Timestamp, Pg>::to_sql(self, out)
    }
}

impl FromSql<types::Timestamptz, Pg> for PgTimestamp {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::Timestamp, Pg>::from_sql(bytes)
    }
}

impl ToSql<types::Date, Pg> for PgDate {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::Integer, Pg>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Date, Pg> for PgDate {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::Integer, Pg>::from_sql(bytes).map(PgDate)
    }
}

impl ToSql<types::Time, Pg> for PgTime {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::BigInt, Pg>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Time, Pg> for PgTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::BigInt, Pg>::from_sql(bytes).map(PgTime)
    }
}

impl ToSql<types::Interval, Pg> for PgInterval {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        try!(ToSql::<types::BigInt, Pg>::to_sql(&self.microseconds, out));
        try!(ToSql::<types::Integer, Pg>::to_sql(&self.days, out));
        try!(ToSql::<types::Integer, Pg>::to_sql(&self.months, out));
        Ok(IsNull::No)
    }
}

impl FromSql<types::Interval, Pg> for PgInterval {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        let bytes = not_none!(bytes);
        Ok(PgInterval {
            microseconds: try!(FromSql::<types::BigInt, Pg>::from_sql(Some(&bytes[..8]))),
            days: try!(FromSql::<types::Integer, Pg>::from_sql(Some(&bytes[8..12]))),
            months: try!(FromSql::<types::Integer, Pg>::from_sql(
                Some(&bytes[12..16])
            )),
        })
    }
}

impl Add<PgInterval> for PgInterval {
    type Output = PgInterval;

    fn add(self, other: PgInterval) -> Self::Output {
        PgInterval {
            microseconds: self.microseconds + other.microseconds,
            days: self.days + other.days,
            months: self.months + other.months,
        }
    }
}
