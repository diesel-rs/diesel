//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.
extern crate chrono;

use std::error::Error;
use std::io::Write;
use self::chrono::{Duration, NaiveDateTime, NaiveDate};
use self::chrono::naive::date;

use expression::AsExpression;
use expression::bound::Bound;
use query_source::Queriable;
use super::{PgDate, PgTimestamp};
use types::{self, FromSql, IsNull, Timestamp, Date, ToSql};

expression_impls! {
    Date -> NaiveDate,
    Timestamp -> NaiveDateTime,
}

queriable_impls! {
    Date -> NaiveDate,
    Timestamp -> NaiveDateTime,
}

fn pg_epoch() -> NaiveDate {
    NaiveDate::from_ymd(2000, 1, 1)
}

fn julian_epoch() -> NaiveDate {
    NaiveDate::from_ymd(-4713, 1, 1)
}

impl FromSql<Date> for NaiveDate {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let PgDate(offset) = try!(FromSql::<Date>::from_sql(bytes));
        match pg_epoch().checked_add(Duration::days(offset as i64)) {
            Some(date) => Ok(date),
            None => {
                let error_message = format!("Chrono can only represent dates up to {:?}", date::MAX);
                Err(Box::<Error + Send + Sync>::from(error_message))
            }
        }
    }
}

impl FromSql<Timestamp> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let PgTimestamp(offset) = try!(FromSql::<Timestamp>::from_sql(bytes));
        Ok(pg_epoch().and_hms(0, 0, 0) + Duration::microseconds(offset))
    }
}

impl ToSql<Date> for NaiveDate {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        if *self < julian_epoch() {
            let error_message = format!("Cannot store dates earlier than {:?}", julian_epoch());
            return Err(Box::<Error + Send + Sync>::from(error_message));
        };
        let days_since_epoch = (*self - pg_epoch()).num_days();
        ToSql::<Date>::to_sql(&PgDate(days_since_epoch as i32), out)
    }
}

impl ToSql<Timestamp> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let time = match (*self - pg_epoch().and_hms(0, 0, 0)).num_microseconds() {
            Some(time) => time,
            None => {
                let error_message = format!("{:?} as microseconds is too large to fit in an i64", self);
                return Err(Box::<Error + Send + Sync>::from(error_message));
            }
        };
        ToSql::<Timestamp>::to_sql(&PgTimestamp(time), out)
    }
}
