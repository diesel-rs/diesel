//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.
extern crate chrono;

use std::error::Error;
use std::io::Write;
use self::chrono::{Duration, NaiveDateTime, NaiveDate, NaiveTime};
use self::chrono::naive::date;

use expression::AsExpression;
use expression::bound::Bound;
use query_source::Queryable;
use super::{PgDate, PgTime, PgTimestamp};
use types::{self, Date, FromSql, IsNull, Time, Timestamp, ToSql};

expression_impls! {
    Date -> NaiveDate,
    Time -> NaiveTime,
    Timestamp -> NaiveDateTime,
}

queryable_impls! {
    Date -> NaiveDate,
    Time -> NaiveTime,
    Timestamp -> NaiveDateTime,
}

// Postgres timestamps start from January 1st 2000.
fn pg_epoch() -> NaiveDateTime {
    NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0)
}

impl FromSql<Timestamp> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let PgTimestamp(offset) = try!(FromSql::<Timestamp>::from_sql(bytes));
        Ok(pg_epoch() + Duration::microseconds(offset))
    }
}

impl ToSql<Timestamp> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let time = match (*self - pg_epoch()).num_microseconds() {
            Some(time) => time,
            None => {
                let error_message = format!("{:?} as microseconds is too large to fit in an i64", self);
                return Err(Box::<Error + Send + Sync>::from(error_message));
            }
        };
        ToSql::<Timestamp>::to_sql(&PgTimestamp(time), out)
    }
}

fn midnight() -> NaiveTime {
    NaiveTime::from_hms(0, 0, 0)
}

impl ToSql<Time> for NaiveTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let duration = *self - midnight();
        match duration.num_microseconds() {
            Some(offset) => ToSql::<Time>::to_sql(&PgTime(offset), out),
            None => unreachable!()
        }
    }
}

impl FromSql<Time> for NaiveTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let PgTime(offset) = try!(FromSql::<Time>::from_sql(bytes));
        let duration = Duration::microseconds(offset);
        Ok(midnight() + duration)
    }
}

fn pg_epoch_date() -> NaiveDate {
    NaiveDate::from_ymd(2000, 1, 1)
}

impl ToSql<Date> for NaiveDate {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let days_since_epoch = (*self - pg_epoch_date()).num_days();
        ToSql::<Date>::to_sql(&PgDate(days_since_epoch as i32), out)
    }
}

impl FromSql<Date> for NaiveDate {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let PgDate(offset) = try!(FromSql::<Date>::from_sql(bytes));
        match pg_epoch_date().checked_add(Duration::days(offset as i64)) {
            Some(date) => Ok(date),
            None => {
                let error_message = format!("Chrono can only represent dates up to {:?}",
                                            date::MAX);
                Err(Box::<Error + Send + Sync>::from(error_message))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;
    extern crate chrono;

    use self::chrono::{Duration, NaiveDate, NaiveTime, UTC};
    use self::chrono::naive::date;
    use self::dotenv::dotenv;

    use ::select;
    use connection::{Connection, PgConnection};
    use expression::dsl::{sql, now};
    use prelude::*;
    use types::{Date, Time, Timestamp};

    fn connection() -> PgConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("DATABASE_URL").ok()
            .expect("DATABASE_URL must be set in order to run tests");
        PgConnection::establish(&connection_url).unwrap()
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql = select(sql::<Timestamp>("'1970-01-01'::timestamp"))
            .get_result(&connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = UTC::now().naive_utc() + Duration::seconds(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = UTC::now().naive_utc() - Duration::seconds(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = connection();

        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(sql::<Time>("'00:00:00'::time").eq(midnight));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("'12:00:00'::time").eq(noon));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 04, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'::time").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = connection();
        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(sql::<Time>("'00:00:00'::time"));
        assert_eq!(Ok(midnight), query.get_result::<NaiveTime>(&connection));

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("'12:00:00'::time"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(&connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 04, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'::time"));
        assert_eq!(Ok(roughly_half_past_eleven), query.get_result::<NaiveTime>(&connection));
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'").eq(january_first_2000));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_past = NaiveDate::from_ymd(-398, 4, 11); // year 0 is 1 BC in this function
        let query = select(sql::<Date>("'399-4-11 BC'").eq(distant_past));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let julian_epoch = NaiveDate::from_ymd(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date").eq(julian_epoch));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let max_date = date::MAX;
        let query = select(sql::<Date>("'262143-12-31'::date").eq(max_date));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_future = NaiveDate::from_ymd(72400, 1, 8);
        let query = select(sql::<Date>("'72400-1-8'::date").eq(distant_future));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'::date"));
        assert_eq!(Ok(january_first_2000), query.get_result::<NaiveDate>(&connection));

        let distant_past = NaiveDate::from_ymd(-398, 4, 11);
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(&connection));

        let julian_epoch = NaiveDate::from_ymd(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(Ok(julian_epoch), query.get_result::<NaiveDate>(&connection));

        let max_date = date::MAX;
        let query = select(sql::<Date>("'262143-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result::<NaiveDate>(&connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(Ok(january_first_2018), query.get_result::<NaiveDate>(&connection));

        let distant_future = NaiveDate::from_ymd(72400, 1, 8);
        let query = select(sql::<Date>("'72400-1-8'::date"));
        assert_eq!(Ok(distant_future), query.get_result::<NaiveDate>(&connection));
    }
}
