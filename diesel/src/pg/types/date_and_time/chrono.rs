//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.

extern crate chrono;

use self::chrono::naive::MAX_DATE;
use self::chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use std::io::Write;

use super::{PgDate, PgTime, PgTimestamp};
use deserialize::{self, FromSql};
use pg::Pg;
use serialize::{self, Output, ToSql};
use sql_types::{Date, Time, Timestamp, Timestamptz};

// Postgres timestamps start from January 1st 2000.
fn pg_epoch() -> NaiveDateTime {
    NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0)
}

impl FromSql<Timestamp, Pg> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let PgTimestamp(offset) = FromSql::<Timestamp, Pg>::from_sql(bytes)?;
        match pg_epoch().checked_add_signed(Duration::microseconds(offset)) {
            Some(v) => Ok(v),
            None => {
                let message = "Tried to deserialize a timestamp that is too large for Chrono";
                Err(message.into())
            }
        }
    }
}

impl ToSql<Timestamp, Pg> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let time = match (self.signed_duration_since(pg_epoch())).num_microseconds() {
            Some(time) => time,
            None => {
                let error_message =
                    format!("{:?} as microseconds is too large to fit in an i64", self);
                return Err(error_message.into());
            }
        };
        ToSql::<Timestamp, Pg>::to_sql(&PgTimestamp(time), out)
    }
}

impl FromSql<Timestamptz, Pg> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        FromSql::<Timestamp, Pg>::from_sql(bytes)
    }
}

impl ToSql<Timestamptz, Pg> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        ToSql::<Timestamp, Pg>::to_sql(self, out)
    }
}

impl FromSql<Timestamptz, Pg> for DateTime<Utc> {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let naive_date_time = <NaiveDateTime as FromSql<Timestamptz, Pg>>::from_sql(bytes)?;
        Ok(DateTime::from_utc(naive_date_time, Utc))
    }
}

impl<TZ: TimeZone> ToSql<Timestamptz, Pg> for DateTime<TZ> {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        ToSql::<Timestamptz, Pg>::to_sql(&self.naive_utc(), out)
    }
}

fn midnight() -> NaiveTime {
    NaiveTime::from_hms(0, 0, 0)
}

impl ToSql<Time, Pg> for NaiveTime {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let duration = self.signed_duration_since(midnight());
        match duration.num_microseconds() {
            Some(offset) => ToSql::<Time, Pg>::to_sql(&PgTime(offset), out),
            None => unreachable!(),
        }
    }
}

impl FromSql<Time, Pg> for NaiveTime {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let PgTime(offset) = FromSql::<Time, Pg>::from_sql(bytes)?;
        let duration = Duration::microseconds(offset);
        Ok(midnight() + duration)
    }
}

fn pg_epoch_date() -> NaiveDate {
    NaiveDate::from_ymd(2000, 1, 1)
}

impl ToSql<Date, Pg> for NaiveDate {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let days_since_epoch = self.signed_duration_since(pg_epoch_date()).num_days();
        ToSql::<Date, Pg>::to_sql(&PgDate(days_since_epoch as i32), out)
    }
}

impl FromSql<Date, Pg> for NaiveDate {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let PgDate(offset) = FromSql::<Date, Pg>::from_sql(bytes)?;
        match pg_epoch_date().checked_add_signed(Duration::days(i64::from(offset))) {
            Some(date) => Ok(date),
            None => {
                let error_message = format!("Chrono can only represent dates up to {:?}", MAX_DATE);
                Err(error_message.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate chrono;
    extern crate dotenv;

    use self::chrono::naive::MAX_DATE;
    use self::chrono::{Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
    use self::dotenv::dotenv;

    use dsl::{now, sql};
    use prelude::*;
    use select;
    use sql_types::{Date, Time, Timestamp, Timestamptz};

    fn connection() -> PgConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("PG_DATABASE_URL")
            .or_else(|_| ::std::env::var("DATABASE_URL"))
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
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = connection();
        let time = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = connection();
        let time = FixedOffset::west(3600).ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(sql::<Timestamptz>("'1970-01-01 01:00:00Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql =
            select(sql::<Timestamp>("'1970-01-01'::timestamp")).get_result(&connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn unix_epoch_decodes_correctly_with_timezone() {
        let connection = connection();
        let time = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql =
            select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz")).get_result(&connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = Utc::now().naive_utc() + Duration::seconds(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = Utc::now().naive_utc() - Duration::seconds(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_with_timezones_round_trip_after_conversion() {
        let connection = connection();
        let time = FixedOffset::east(3600).ymd(2016, 1, 2).and_hms(1, 0, 0);
        let expected = NaiveDate::from_ymd(2016, 1, 1).and_hms(20, 0, 0);
        let query = select(time.into_sql::<Timestamptz>().at_time_zone("EDT"));
        assert_eq!(Ok(expected), query.get_result(&connection));
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

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
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

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'::time"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(&connection)
        );
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

        let max_date = MAX_DATE;
        let query = select(sql::<Date>("'262143-12-31'::date").eq(max_date));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_future = NaiveDate::from_ymd(72_400, 1, 8);
        let query = select(sql::<Date>("'72400-1-8'::date").eq(distant_future));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'::date"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(&connection)
        );

        let distant_past = NaiveDate::from_ymd(-398, 4, 11);
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(&connection));

        let julian_epoch = NaiveDate::from_ymd(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(Ok(julian_epoch), query.get_result::<NaiveDate>(&connection));

        let max_date = MAX_DATE;
        let query = select(sql::<Date>("'262143-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result::<NaiveDate>(&connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(&connection)
        );

        let distant_future = NaiveDate::from_ymd(72_400, 1, 8);
        let query = select(sql::<Date>("'72400-1-8'::date"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDate>(&connection)
        );
    }
}
