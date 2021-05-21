extern crate chrono;

use self::chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use std::io::Write;

use crate::backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Text, Time, Timestamp};
use crate::sqlite::Sqlite;

const SQLITE_DATE_FORMAT: &str = "%F";

impl FromSql<Date, Sqlite> for NaiveDate {
    fn from_sql(value: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let text_ptr = <*const str as FromSql<Date, Sqlite>>::from_sql(value)?;
        let text = unsafe { &*text_ptr };
        Self::parse_from_str(text, SQLITE_DATE_FORMAT).map_err(Into::into)
    }
}

impl ToSql<Date, Sqlite> for NaiveDate {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let s = self.format(SQLITE_DATE_FORMAT).to_string();
        ToSql::<Text, Sqlite>::to_sql(&s, out)
    }
}

impl FromSql<Time, Sqlite> for NaiveTime {
    fn from_sql(value: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let text_ptr = <*const str as FromSql<Date, Sqlite>>::from_sql(value)?;
        let text = unsafe { &*text_ptr };
        let valid_time_formats = &[
            // Most likely
            "%T%.f", // All other valid formats in order of documentation
            "%R", "%RZ", "%T%.fZ", "%R%:z", "%T%.f%:z",
        ];

        for format in valid_time_formats {
            if let Ok(time) = Self::parse_from_str(text, format) {
                return Ok(time);
            }
        }

        Err(format!("Invalid time {}", text).into())
    }
}

impl ToSql<Time, Sqlite> for NaiveTime {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let s = self.format("%T%.f").to_string();
        ToSql::<Text, Sqlite>::to_sql(&s, out)
    }
}

impl FromSql<Timestamp, Sqlite> for NaiveDateTime {
    fn from_sql(value: backend::RawValue<Sqlite>) -> deserialize::Result<Self> {
        let text_ptr = <*const str as FromSql<Date, Sqlite>>::from_sql(value)?;
        let text = unsafe { &*text_ptr };

        let sqlite_datetime_formats = &[
            // Most likely format
            "%F %T%.f",
            // Other formats in order of appearance in docs
            "%F %R",
            "%F %RZ",
            "%F %R%:z",
            "%F %T%.fZ",
            "%F %T%.f%:z",
            "%FT%R",
            "%FT%RZ",
            "%FT%R%:z",
            "%FT%T%.f",
            "%FT%T%.fZ",
            "%FT%T%.f%:z",
        ];

        for format in sqlite_datetime_formats {
            if let Ok(dt) = Self::parse_from_str(text, format) {
                return Ok(dt);
            }
        }

        if let Ok(julian_days) = text.parse::<f64>() {
            let epoch_in_julian_days = 2_440_587.5;
            let seconds_in_day = 86400.0;
            let timestamp = (julian_days - epoch_in_julian_days) * seconds_in_day;
            let seconds = timestamp as i64;
            let nanos = (timestamp.fract() * 1E9) as u32;
            if let Some(timestamp) = Self::from_timestamp_opt(seconds, nanos) {
                return Ok(timestamp);
            }
        }

        Err(format!("Invalid datetime {}", text).into())
    }
}

impl ToSql<Timestamp, Sqlite> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let s = self.format("%F %T%.f").to_string();
        ToSql::<Text, Sqlite>::to_sql(&s, out)
    }
}

// chrono::DateTime<FixedOffset> impls

impl FromSql<Text, Sqlite> for DateTime<FixedOffset> {
    fn from_sql(value: Option<backend::RawValue<Sqlite>>) -> deserialize::Result<Self> {
        let text_ptr = <*const str as FromSql<Text, Sqlite>>::from_sql(value)?;
        let text = unsafe { &*text_ptr };

        Self::parse_from_rfc3339(&text).map_err(Into::into)
    }
}

impl ToSql<Text, Sqlite> for DateTime<FixedOffset> {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let s = self.to_rfc3339();
        ToSql::<Text, Sqlite>::to_sql(&s, out)
    }
}


#[cfg(test)]
mod tests {
    extern crate chrono;
    extern crate dotenv;

    use self::chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike, TimeZone, Utc};
    use self::dotenv::dotenv;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Text, Time, Timestamp};

    sql_function!(fn datetime(x: Text) -> Timestamp);
    sql_function!(fn time(x: Text) -> Time);
    sql_function!(fn date(x: Text) -> Date);

    fn connection() -> SqliteConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("SQLITE_DATABASE_URL")
            .or_else(|_| ::std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        SqliteConnection::establish(&connection_url).unwrap()
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(datetime("1970-01-01 00:00:00.000000").eq(time));
        assert_eq!(Ok(true), query.get_result(&connection));
    }

    #[test]
    fn unix_epoch_decodes_correctly_in_all_possible_formats() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let valid_epoch_formats = vec![
            "1970-01-01 00:00",
            "1970-01-01 00:00:00",
            "1970-01-01 00:00:00.000",
            "1970-01-01 00:00:00.000000",
            "1970-01-01T00:00",
            "1970-01-01T00:00:00",
            "1970-01-01T00:00:00.000",
            "1970-01-01T00:00:00.000000",
            "1970-01-01 00:00Z",
            "1970-01-01 00:00:00Z",
            "1970-01-01 00:00:00.000Z",
            "1970-01-01 00:00:00.000000Z",
            "1970-01-01T00:00Z",
            "1970-01-01T00:00:00Z",
            "1970-01-01T00:00:00.000Z",
            "1970-01-01T00:00:00.000000Z",
            "1970-01-01 00:00+00:00",
            "1970-01-01 00:00:00+00:00",
            "1970-01-01 00:00:00.000+00:00",
            "1970-01-01 00:00:00.000000+00:00",
            "1970-01-01T00:00+00:00",
            "1970-01-01T00:00:00+00:00",
            "1970-01-01T00:00:00.000+00:00",
            "1970-01-01T00:00:00.000000+00:00",
            "1970-01-01 00:00+01:00",
            "1970-01-01 00:00:00+01:00",
            "1970-01-01 00:00:00.000+01:00",
            "1970-01-01 00:00:00.000000+01:00",
            "1970-01-01T00:00+01:00",
            "1970-01-01T00:00:00+01:00",
            "1970-01-01T00:00:00.000+01:00",
            "1970-01-01T00:00:00.000000+01:00",
            "1970-01-01T00:00-01:00",
            "1970-01-01T00:00:00-01:00",
            "1970-01-01T00:00:00.000-01:00",
            "1970-01-01T00:00:00.000000-01:00",
            "1970-01-01T00:00-01:00",
            "1970-01-01T00:00:00-01:00",
            "1970-01-01T00:00:00.000-01:00",
            "1970-01-01T00:00:00.000000-01:00",
            "2440587.5",
        ];

        for s in valid_epoch_formats {
            let epoch_from_sql =
                select(sql::<Timestamp>(&format!("'{}'", s))).get_result(&connection);
            assert_eq!(Ok(time), epoch_from_sql, "format {} failed", s);
        }
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = Utc::now().naive_utc() + Duration::seconds(60);
        let query = select(now.lt(time));
        assert_eq!(Ok(true), query.get_result(&connection));

        let time = Utc::now().naive_utc() - Duration::seconds(600);
        let query = select(now.gt(time));
        assert_eq!(Ok(true), query.get_result(&connection));
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = connection();

        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(time("00:00:00.000000").eq(midnight));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(time("12:00:00.000000").eq(noon));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = connection();
        let midnight = NaiveTime::from_hms(0, 0, 0);
        let valid_midnight_formats = &[
            "00:00",
            "00:00:00",
            "00:00:00.000",
            "00:00:00.000000",
            "00:00Z",
            "00:00:00Z",
            "00:00:00.000Z",
            "00:00:00.000000Z",
            "00:00+00:00",
            "00:00:00+00:00",
            "00:00:00.000+00:00",
            "00:00:00.000000+00:00",
            "00:00+01:00",
            "00:00:00+01:00",
            "00:00:00.000+01:00",
            "00:00:00.000000+01:00",
            "00:00-01:00",
            "00:00:00-01:00",
            "00:00:00.000-01:00",
            "00:00:00.000000-01:00",
        ];
        for format in valid_midnight_formats {
            let query = select(sql::<Time>(&format!("'{}'", format)));
            assert_eq!(
                Ok(midnight),
                query.get_result::<NaiveTime>(&connection),
                "format {} failed",
                format
            );
        }

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("'12:00:00'"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(&connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(&connection)
        );
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(date("2000-01-01").eq(january_first_2000));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_past = NaiveDate::from_ymd(0, 4, 11);
        let query = select(date("0000-04-11").eq(distant_past));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(date("2018-01-01").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_future = NaiveDate::from_ymd(9999, 1, 8);
        let query = select(date("9999-01-08").eq(distant_future));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(date("2000-01-01"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(&connection)
        );

        let distant_past = NaiveDate::from_ymd(0, 4, 11);
        let query = select(date("0000-04-11"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(&connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(date("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(&connection)
        );

        let distant_future = NaiveDate::from_ymd(9999, 1, 8);
        let query = select(date("9999-01-08"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDate>(&connection)
        );
    }

    #[test]
    fn datetimes_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1).and_hms(1, 1, 1);
        let query = select(datetime("2000-01-01 01:01:01.000000"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDateTime>(&connection)
        );

        let distant_past = NaiveDate::from_ymd(0, 4, 11).and_hms(2, 2, 2);
        let query = select(datetime("0000-04-11 02:02:02.000000"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<NaiveDateTime>(&connection)
        );

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(date("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(&connection)
        );

        let distant_future = NaiveDate::from_ymd(9999, 1, 8)
            .and_hms(23, 59, 59)
            .with_nanosecond(100_000)
            .unwrap();
        let query = select(sql::<Timestamp>("'9999-01-08 23:59:59.000100'"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDateTime>(&connection)
        );
    }

    #[test]
    fn datetimes_encode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);
        let query = select(datetime("2000-01-01 00:00:00.000000").eq(january_first_2000));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_past = NaiveDate::from_ymd(0, 4, 11).and_hms(20, 00, 20);
        let query = select(datetime("0000-04-11 20:00:20.000000").eq(distant_past));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1)
            .and_hms(12, 00, 00)
            .with_nanosecond(500_000)
            .unwrap();
        let query = select(sql::<Timestamp>("'2018-01-01 12:00:00.000500'").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_future = NaiveDate::from_ymd(9999, 1, 8).and_hms(0, 0, 0);
        let query = select(datetime("9999-01-08 00:00:00.000000").eq(distant_future));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    // chrono::DateTime decode/encode tests

    #[test]
    /// Test that a rfc3339-formatted chrono::DateTime decodes correctly.
    fn rfc3339_datetime_decodes_correctly() {
        let connection = connection();

        // from datetime_decodes_correctly for NaiveDateTime, but with Utc timezone

        let january_first_2000: DateTime<FixedOffset> = Utc
            .ymd(2000, 1, 1)
            .and_hms(1, 1, 1).into();
        let query = select(sql::<Text>("'2000-01-01T01:01:01.000000Z'"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        let distant_past: DateTime<FixedOffset> = Utc
            .ymd(0, 4, 11)
            .and_hms(2, 2, 2).into();
        let query = select(sql::<Text>("'0000-04-11t02:02:02.000000Z'"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        let january_first_2018: DateTime<FixedOffset> = Utc
            .ymd(2018, 1, 1)
            .and_hms(1, 1, 1).into();
        let query = select(sql::<Text>("'2018-01-01T01:01:01.00Z'"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        let distant_future: DateTime<FixedOffset> = Utc
            .ymd(9999, 1, 8)
            .and_hms_nano(23, 59, 59, 100_000).into();
        let query = select(sql::<Text>("'9999-01-08t23:59:59.000100z'"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        // same test cases as above with different timezones
        let hour: i32 = 3600;

        let january_first_2000: DateTime<FixedOffset> = FixedOffset::east(hour * 4)
            .ymd(2000, 1, 1)
            .and_hms(1, 1, 1).into();
        let query = select(sql::<Text>("'2000-01-01T01:01:01.0+04:00'"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        let distant_past: DateTime<FixedOffset> = FixedOffset::east(hour * -3)
            .ymd(0, 4, 11)
            .and_hms(2, 2, 2).into();
        let query = select(sql::<Text>("'0000-04-11t02:02:02.000000-03:00'"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        let january_first_2018: DateTime<FixedOffset> = FixedOffset::east(hour * 12)
            .ymd(2018, 1, 1)
            .and_hms(1, 1, 1).into();
        let query = select(sql::<Text>("'2018-01-01T01:01:01.00+12:00'"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );

        let distant_future: DateTime<FixedOffset> = FixedOffset::east(hour * -7 + (hour/2))
            .ymd(9999, 1, 8)
            .and_hms_nano(23, 59, 59, 100_000).into();
        let query = select(sql::<Text>("'9999-01-08t23:59:59.000100-06:30'"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<DateTime<FixedOffset>>(&connection)
        );
    }

    #[test]
    /// Test that an incorrectly-formatted rfc3339 timestamp decodes with an error.
    fn invalid_rfc3339_datetime_decodes_error() {
        let connection = connection();

        let query = select(sql::<Text>("''"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_err(), "Empty timestamp was parsed into valid datetime: {}", res.unwrap().to_string());

        let query = select(sql::<Text>("'x'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_err(), "Invalid timestamp was parsed into valid datetime: {}", res.unwrap().to_string());

        let query = select(sql::<Text>("'x'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_err(), "Invalid timestamp was parsed into valid datetime: {}", res.unwrap().to_string());

        let query = select(sql::<Text>("'9999-01-08 23:59:59.000100'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_err(), "Timestamp with no timezone was parsed into valid datetime: {}", res.unwrap().to_string());

        let query = select(sql::<Text>("'2000-01-01T01:01:01.000000+25:00'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_err(), "Timestamp with invalid timezone was parsed into valid datetime: {}", res.unwrap().to_string());

        let query = select(sql::<Text>("'Wed, 18 Feb 2015 23:16:09 GMT'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_err(), "RFC 2822 timestamp (lol) was parsed into valid datetime: {}", res.unwrap().to_string());
    }

    #[test]
    /// Test rfc3339 datetimes encode correctly. Fewer test cases here because we're literally just
    /// storing it as a string.
    fn rfc3339_datetimes_encode_correctly() {
        let connection = connection();
        let january_first_2000: DateTime<FixedOffset> = Utc
            .ymd(2000, 1, 1)
            .and_hms(0, 0, 0).into();

        // select the rfc3339 string as text, extract the result as a chrono::DateTime, check no
        // error, and then check that it deserialized to the correct datetime. All of the tests
        // below follow this pattern.
        let query = select(sql::<Text>("'2000-01-01T00:00:00.000000Z'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_ok(), "Error selecting datetime: {}", res.unwrap_err());

        let dt = res.unwrap();
        assert_eq!(dt, january_first_2000);


        // same as above, checking that regardless of timezone storage (Z or +/-00:00), the resulting DateTime is
        // the same.
        let query = select(sql::<Text>("'2000-01-01T00:00:00.000000+00:00'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_ok(), "Error selecting datetime: {}", res.unwrap_err());

        let dt = res.unwrap();
        assert_eq!(dt, january_first_2000);


        let query = select(sql::<Text>("'2000-01-01T00:00:00.000000-00:00'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_ok(), "Error selecting datetime: {}", res.unwrap_err());

        let dt = res.unwrap();
        assert_eq!(dt, january_first_2000);


        let distant_past: DateTime<FixedOffset> = Utc
            .ymd(0, 4, 11)
            .and_hms(20, 00, 20).into();
        let query = select(sql::<Text>("'0000-04-11t20:00:20.000000Z'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_ok(), "Error selecting datetime: {}", res.unwrap_err());

        let dt = res.unwrap();
        assert_eq!(dt, distant_past);


        let january_first_2018 = FixedOffset::east(30*60)
            .ymd(2018, 1, 1)
            .and_hms_nano(12, 00, 00, 500_000);
        let query = select(sql::<Text>("'2018-01-01t12:00:00.000500+00:30'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_ok(), "Error selecting datetime: {}", res.unwrap_err());

        let dt = res.unwrap();
        assert_eq!(dt, january_first_2018);


        let distant_future = FixedOffset::west(3600)
            .ymd(9999, 1, 8)
            .and_hms(0, 0, 0);
        let query = select(sql::<Text>("'9999-01-08t00:00:00.000000-01:00'"));
        let res = query.get_result::<DateTime<FixedOffset>>(&connection);
        assert!(res.is_ok(), "Error selecting datetime: {}", res.unwrap_err());

        let dt = res.unwrap();
        assert_eq!(dt, distant_future);
    }

    // #[test]
    // /// Test that the sql select...where ordering functionality interops properly with rfc3339 datetimes.
    // fn rfc3339_datetimes_relative_to_now_encode_correctly() {
    //     let connection = connection();
    //     let time: DateTime<FixedOffset> = (Utc::now() + Duration::seconds(60)).into();
    //     let query = select(now.lt(time));
    //     assert_eq!(Ok(true), query.get_result(&connection));

    //     let time: DateTime<FixedOffset> = (Utc::now() - Duration::seconds(60)).into();
    //     let query = select(now.gt(time));
    //     assert_eq!(Ok(true), query.get_result(&connection));
    // }

    // #[test]
    // fn rfc3339_datetimes_order_by_properly() {
    //     let connection = connection();
    //     let now = Utc::now();
    //     let now_est = Utc::now().with_timezone(FixedOffset::west(3600 * 5));
    //     let past: DateTime<FixedOffset> = (now - Duration::seconds(60)).into();
    //     let future: DateTime<FixedOffset> = (now + Duration::seconds(60)).into();

    //     // TODO: how do I select multiple row/value literals to test order_by?
    //     // select "literal1" UNION ALL select "literal2" [...]?

    // }
}
