extern crate chrono;
use self::chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

use crate::backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Date, Time, Timestamp, TimestamptzSqlite};
use crate::sqlite::Sqlite;

const SQLITE_DATE_FORMAT: &str = "%F";

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl FromSql<Date, Sqlite> for NaiveDate {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value
            .parse_string(|s| Self::parse_from_str(s, SQLITE_DATE_FORMAT))
            .map_err(Into::into)
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl ToSql<Date, Sqlite> for NaiveDate {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.format(SQLITE_DATE_FORMAT).to_string());
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl FromSql<Time, Sqlite> for NaiveTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
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
        })
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl ToSql<Time, Sqlite> for NaiveTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.format("%T%.f").to_string());
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl FromSql<Timestamp, Sqlite> for NaiveDateTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
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
        })
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl ToSql<Timestamp, Sqlite> for NaiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.format("%F %T%.f").to_string());
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl FromSql<TimestamptzSqlite, Sqlite> for NaiveDateTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            let sqlite_datetime_formats = &[
                "%F %RZ",
                "%F %R%:z",
                "%F %TZ",
                "%F %T%:z",
                "%F %T%.fZ",
                "%F %T%.f%:z",
                "%FT%RZ",
                "%FT%R%:z",
                "%FT%TZ",
                "%FT%T%:z",
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
        })
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl ToSql<TimestamptzSqlite, Sqlite> for NaiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.format("%F %T%.f+00:00").to_string());
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl FromSql<TimestamptzSqlite, Sqlite> for DateTime<Utc> {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        let naive_date_time =
            <NaiveDateTime as FromSql<TimestamptzSqlite, Sqlite>>::from_sql(value)?;
        Ok(DateTime::from_utc(naive_date_time, Utc))
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl FromSql<TimestamptzSqlite, Sqlite> for DateTime<Local> {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        let naive_date_time =
            <NaiveDateTime as FromSql<TimestamptzSqlite, Sqlite>>::from_sql(value)?;
        Ok(Local::from_utc_datetime(&Local, &naive_date_time))
    }
}

#[cfg(all(feature = "sqlite", feature = "chrono"))]
impl<TZ: TimeZone> ToSql<TimestamptzSqlite, Sqlite> for DateTime<TZ> {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.naive_utc().format("%F %T%.f+00:00").to_string());
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    extern crate chrono;
    extern crate dotenvy;

    use self::chrono::{
        DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike,
        Utc,
    };

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Text, Time, Timestamp, TimestamptzSqlite};
    use crate::test_helpers::connection;

    sql_function!(fn datetime(x: Text) -> Timestamp);
    sql_function!(fn time(x: Text) -> Time);
    sql_function!(fn date(x: Text) -> Date);

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(datetime("1970-01-01 00:00:00.000000").eq(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    #[test]
    fn unix_epoch_decodes_correctly_in_all_possible_formats() {
        let connection = &mut connection();
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
                select(sql::<Timestamp>(&format!("'{}'", s))).get_result(connection);
            assert_eq!(Ok(time), epoch_from_sql, "format {} failed", s);
        }
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = Utc::now().naive_utc() + Duration::seconds(60);
        let query = select(now.lt(time));
        assert_eq!(Ok(true), query.get_result(connection));

        let time = Utc::now().naive_utc() - Duration::seconds(600);
        let query = select(now.gt(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(time("00:00:00.000000").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(time("12:00:00.000000").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
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
                query.get_result::<NaiveTime>(connection),
                "format {} failed",
                format
            );
        }

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("'12:00:00'"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(connection)
        );
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(date("2000-01-01").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = NaiveDate::from_ymd(0, 4, 11);
        let query = select(date("0000-04-11").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(date("2018-01-01").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = NaiveDate::from_ymd(9999, 1, 8);
        let query = select(date("9999-01-08").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(date("2000-01-01"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_past = NaiveDate::from_ymd(0, 4, 11);
        let query = select(date("0000-04-11"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(date("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_future = NaiveDate::from_ymd(9999, 1, 8);
        let query = select(date("9999-01-08"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDate>(connection)
        );
    }

    #[test]
    fn datetimes_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1).and_hms(1, 1, 1);
        let query = select(datetime("2000-01-01 01:01:01.000000"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDateTime>(connection)
        );

        let distant_past = NaiveDate::from_ymd(0, 4, 11).and_hms(2, 2, 2);
        let query = select(datetime("0000-04-11 02:02:02.000000"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<NaiveDateTime>(connection)
        );

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(date("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_future = NaiveDate::from_ymd(9999, 1, 8)
            .and_hms(23, 59, 59)
            .with_nanosecond(100_000)
            .unwrap();
        let query = select(sql::<Timestamp>("'9999-01-08 23:59:59.000100'"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDateTime>(connection)
        );
    }

    #[test]
    fn datetimes_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);
        let query = select(datetime("2000-01-01 00:00:00.000000").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = NaiveDate::from_ymd(0, 4, 11).and_hms(20, 00, 20);
        let query = select(datetime("0000-04-11 20:00:20.000000").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1)
            .and_hms(12, 00, 00)
            .with_nanosecond(500_000)
            .unwrap();
        let query = select(sql::<Timestamp>("'2018-01-01 12:00:00.000500'").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = NaiveDate::from_ymd(9999, 1, 8).and_hms(0, 0, 0);
        let query = select(datetime("9999-01-08 00:00:00.000000").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn insert_timestamptz_into_table_as_text() {
        crate::table! {
            #[allow(unused_parens)]
            test_insert_timestamptz_into_table_as_text(id) {
                id -> Integer,
                timestamp_with_tz -> TimestamptzSqlite,
            }
        }
        let conn = &mut connection();
        crate::sql_query(
            "CREATE TABLE test_insert_timestamptz_into_table_as_text(id INTEGER PRIMARY KEY, timestamp_with_tz TEXT);",
        )
        .execute(conn)
        .unwrap();

        let time: DateTime<Utc> = Utc.ymd(1970, 1, 1).and_hms_milli(0, 0, 0, 0);

        crate::insert_into(test_insert_timestamptz_into_table_as_text::table)
            .values(vec![(
                test_insert_timestamptz_into_table_as_text::id.eq(1),
                test_insert_timestamptz_into_table_as_text::timestamp_with_tz.eq(sql::<
                    TimestamptzSqlite,
                >(
                    "'1970-01-01 00:00:00.000000+00:00'",
                )),
            )])
            .execute(conn)
            .unwrap();

        let result = test_insert_timestamptz_into_table_as_text::table
            .select(test_insert_timestamptz_into_table_as_text::timestamp_with_tz)
            .get_result::<DateTime<Utc>>(conn)
            .unwrap();
        assert_eq!(result, time);
    }

    #[test]
    fn can_query_timestamptz_column_with_between() {
        crate::table! {
            #[allow(unused_parens)]
            test_query_timestamptz_column_with_between(id) {
                id -> Integer,
                timestamp_with_tz -> TimestamptzSqlite,
            }
        }
        let conn = &mut connection();
        crate::sql_query(
            "CREATE TABLE test_query_timestamptz_column_with_between(id INTEGER PRIMARY KEY, timestamp_with_tz TEXT);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(test_query_timestamptz_column_with_between::table)
            .values(vec![
                (
                    test_query_timestamptz_column_with_between::id.eq(1),
                    test_query_timestamptz_column_with_between::timestamp_with_tz.eq(sql::<
                        TimestamptzSqlite,
                    >(
                        "'1970-01-01 00:00:01.000000+00:00'",
                    )),
                ),
                (
                    test_query_timestamptz_column_with_between::id.eq(2),
                    test_query_timestamptz_column_with_between::timestamp_with_tz.eq(sql::<
                        TimestamptzSqlite,
                    >(
                        "'1970-01-01 00:00:02.000000+00:00'",
                    )),
                ),
                (
                    test_query_timestamptz_column_with_between::id.eq(3),
                    test_query_timestamptz_column_with_between::timestamp_with_tz.eq(sql::<
                        TimestamptzSqlite,
                    >(
                        "'1970-01-01 00:00:03.000000+00:00'",
                    )),
                ),
                (
                    test_query_timestamptz_column_with_between::id.eq(4),
                    test_query_timestamptz_column_with_between::timestamp_with_tz.eq(sql::<
                        TimestamptzSqlite,
                    >(
                        "'1970-01-01 00:00:04.000000+00:00'",
                    )),
                ),
            ])
            .execute(conn)
            .unwrap();

        let result = test_query_timestamptz_column_with_between::table
            .select(test_query_timestamptz_column_with_between::timestamp_with_tz)
            .filter(
                test_query_timestamptz_column_with_between::timestamp_with_tz
                    .gt(Utc.ymd(1970, 1, 1).and_hms_milli(0, 0, 0, 0)),
            )
            .filter(
                test_query_timestamptz_column_with_between::timestamp_with_tz
                    .lt(Utc.ymd(1970, 1, 1).and_hms_milli(0, 0, 4, 0)),
            )
            .count()
            .get_result::<_>(conn);
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = &mut connection();
        // West one hour is negative offset
        let time = FixedOffset::west(3600)
            .ymd(1970, 1, 1)
            .and_hms_milli(0, 0, 0, 1);
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 01:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = &mut connection();
        let time: DateTime<Utc> = Utc.ymd(1970, 1, 1).and_hms_milli(0, 0, 0, 1);
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 00:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly_with_utc_timezone_in_all_possible_formats() {
        let connection = &mut connection();
        let time: DateTime<Utc> = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let valid_epoch_formats = vec![
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
                select(sql::<TimestamptzSqlite>(&format!("'{}'", s))).get_result(connection);
            assert_eq!(Ok(time), epoch_from_sql, "format {} failed", s);
        }
    }
}
