extern crate byteorder;

use std::error::Error;
use std::io::Write;
use std::ops::Add;

use expression::*;
use expression::bound::Bound;
use query_source::Queriable;
use super::option::UnexpectedNullError;
use types::{self, NativeSqlType, FromSql, ToSql, IsNull};

#[cfg(feature = "quickcheck")]
mod quickcheck_impls;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Timestamps are represented in Postgres as a 32 bit signed integer representing the number of
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
/// microseconds, a 32 bit integer representing number of minutes, and a 32 bit integer
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

primitive_impls! {
    Date -> (PgDate, 1082),
    Interval -> (PgInterval, 1186),
    Time -> (PgTime, 1083),
    Timestamp -> (PgTimestamp, 1114),
}

impl ToSql<types::Timestamp> for PgTimestamp {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::BigInt>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Timestamp> for PgTimestamp {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        FromSql::<types::BigInt>::from_sql(bytes)
            .map(PgTimestamp)
    }
}

impl ToSql<types::Date> for PgDate {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::Integer>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Date> for PgDate {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        FromSql::<types::Integer>::from_sql(bytes)
            .map(PgDate)
    }
}

impl ToSql<types::Time> for PgTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::BigInt>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Time> for PgTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        FromSql::<types::BigInt>::from_sql(bytes)
            .map(PgTime)
    }
}

impl ToSql<types::Interval> for PgInterval {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        try!(ToSql::<types::BigInt>::to_sql(&self.microseconds, out));
        try!(ToSql::<types::Integer>::to_sql(&self.days, out));
        try!(ToSql::<types::Integer>::to_sql(&self.months, out));
        Ok(IsNull::No)
    }
}

impl FromSql<types::Interval> for PgInterval {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let bytes = not_none!(bytes);
        Ok(PgInterval {
            microseconds: try!(FromSql::<types::BigInt>::from_sql(Some(&bytes[..8]))),
            days: try!(FromSql::<types::Integer>::from_sql(Some(&bytes[8..12]))),
            months: try!(FromSql::<types::Integer>::from_sql(Some(&bytes[12..16]))),
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
