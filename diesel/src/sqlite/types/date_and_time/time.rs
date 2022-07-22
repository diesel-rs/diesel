//! This module makes it possible to map `time` date and time values to sqlite `NUMERIC`
//! fields. It is enabled with the `time` feature.

extern crate time;

use self::time::{
    error::ComponentRange, format_description::FormatItem, macros::format_description,
    Date as NaiveDate, OffsetDateTime, PrimitiveDateTime, Time as NaiveTime, UtcOffset,
};

use crate::backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Date, Time, Timestamp, TimestamptzSqlite};
use crate::sqlite::Sqlite;

const DATE_FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");
const ENCODE_TIME_FORMAT: &[FormatItem<'_>] =
    format_description!("[hour]:[minute]:[second].[subsecond digits:6]");

const TIME_FORMATS: [&[FormatItem<'_>]; 9] = [
    // Most likely
    format_description!("[hour]:[minute]:[second].[subsecond]"),
    // All other valid formats in order of increasing specificity
    format_description!("[hour]:[minute]"),
    format_description!("[hour]:[minute]Z"),
    format_description!("[hour]:[minute][offset_hour sign:mandatory]:[offset_minute]"),
    format_description!("[hour]:[minute]:[second]"),
    format_description!("[hour]:[minute]:[second]Z"),
    format_description!("[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"),
    // here is where the most likely format would go according to the pattern
    format_description!("[hour]:[minute]:[second].[subsecond]Z"),
    format_description!(
        "[hour]:[minute]:[second].[subsecond][offset_hour sign:mandatory]:[offset_minute]"
    ),
];

const ENCODE_PRIMITIVE_DATETIME_FORMAT: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]");

const ENCODE_DATETIME_FORMAT: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6][offset_hour sign:mandatory]:[offset_minute]");

const DATETIME_FORMATS: [&[FormatItem<'_>]; 18] = [
    // Most likely format
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"),
    // Other formats in order of increasing specificity
    format_description!("[year]-[month]-[day] [hour]:[minute]"),
    format_description!("[year]-[month]-[day] [hour]:[minute]Z"),
    format_description!("[year]-[month]-[day] [hour]:[minute][offset_hour sign:mandatory]:[offset_minute]"),
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]Z"),
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"),
    // here is where the most likely format would go according to the pattern
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]Z"),
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond][offset_hour sign:mandatory]:[offset_minute]"),
    // now the same thing, with T
    format_description!("[year]-[month]-[day]T[hour]:[minute]"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]Z"),
    format_description!("[year]-[month]-[day]T[hour]:[minute][offset_hour sign:mandatory]:[offset_minute]"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]Z"),
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond][offset_hour sign:mandatory]:[offset_minute]"),
];

fn naive_utc(dt: OffsetDateTime) -> PrimitiveDateTime {
    let dt = dt.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(dt.date(), dt.time())
}

fn parse_julian(julian_days: f64) -> Result<PrimitiveDateTime, ComponentRange> {
    const EPOCH_IN_JULIAN_DAYS: f64 = 2_440_587.5;
    const SECONDS_IN_DAY: f64 = 86400.0;
    let timestamp = (julian_days - EPOCH_IN_JULIAN_DAYS) * SECONDS_IN_DAY;
    OffsetDateTime::from_unix_timestamp_nanos((timestamp * 1E9) as i128)
        .map(|timestamp| naive_utc(timestamp))
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl FromSql<Date, Sqlite> for NaiveDate {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value
            .parse_string(|s| Self::parse(s, DATE_FORMAT))
            .map_err(Into::into)
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl ToSql<Date, Sqlite> for NaiveDate {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(dbg!(self
            .format(DATE_FORMAT)
            .map_err(|err| err.to_string()))?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl FromSql<Time, Sqlite> for NaiveTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            for format in TIME_FORMATS {
                if let Ok(time) = Self::parse(text, format) {
                    return Ok(time);
                }
            }

            Err(format!("Invalid time {}", text).into())
        })
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl ToSql<Time, Sqlite> for NaiveTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(dbg!(self
            .format(ENCODE_TIME_FORMAT)
            .map_err(|err| err.to_string()))?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl FromSql<Timestamp, Sqlite> for PrimitiveDateTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            for format in DATETIME_FORMATS {
                if let Ok(dt) = Self::parse(text, format) {
                    return Ok(dt);
                }
            }

            if let Ok(julian_days) = text.parse::<f64>() {
                if let Ok(timestamp) = parse_julian(julian_days) {
                    return Ok(timestamp);
                }
            }

            Err(format!("Invalid datetime {}", text).into())
        })
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl ToSql<Timestamp, Sqlite> for PrimitiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(dbg!(self
            .format(ENCODE_PRIMITIVE_DATETIME_FORMAT)
            .map_err(|err| err.to_string()))?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl FromSql<TimestamptzSqlite, Sqlite> for PrimitiveDateTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            for format in DATETIME_FORMATS {
                if let Ok(dt) = Self::parse(text, format) {
                    return Ok(dt);
                }
            }

            if let Ok(julian_days) = text.parse::<f64>() {
                if let Ok(timestamp) = parse_julian(julian_days) {
                    return Ok(timestamp);
                }
            }

            Err(format!("Invalid datetime {}", text).into())
        })
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl ToSql<TimestamptzSqlite, Sqlite> for PrimitiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(dbg!(self
            .format(ENCODE_PRIMITIVE_DATETIME_FORMAT)
            .map_err(|err| err.to_string()))?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl FromSql<TimestamptzSqlite, Sqlite> for OffsetDateTime {
    fn from_sql(value: backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        let primitive_date_time =
            <PrimitiveDateTime as FromSql<TimestamptzSqlite, Sqlite>>::from_sql(value)?;
        Ok(primitive_date_time.assume_utc())
    }
}

#[cfg(all(feature = "sqlite", feature = "time"))]
impl ToSql<TimestamptzSqlite, Sqlite> for OffsetDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let dt_utc = self.to_offset(UtcOffset::UTC);
        out.set_value(dbg!(dt_utc
            .format(ENCODE_DATETIME_FORMAT)
            .map_err(|err| err.to_string()))?);
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use ::time::{
        macros::{date, datetime},
        Date as NaiveDate, Duration, OffsetDateTime, PrimitiveDateTime, Time as NaiveTime,
    };

    use super::naive_utc;

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
        let time = datetime!(1970-1-1 0:0:0);
        let query = select(datetime("1970-01-01 00:00:00.000000").eq(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    #[test]
    fn unix_epoch_decodes_correctly_in_all_possible_formats() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:0:0);
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
        let time = naive_utc(OffsetDateTime::now_utc()) + Duration::seconds(60);
        let query = select(now.lt(time));
        assert_eq!(Ok(true), query.get_result(connection));

        let time = naive_utc(OffsetDateTime::now_utc()) - Duration::seconds(600);
        let query = select(now.gt(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = NaiveTime::from_hms(0, 0, 0).unwrap();
        let query = select(time("00:00:00.000000").eq(midnight));
        assert_eq!(query.get_result::<bool>(connection).unwrap(), true);

        let noon = NaiveTime::from_hms(12, 0, 0).unwrap();
        let query = select(time("12:00:00.000000").eq(noon));
        assert_eq!(query.get_result::<bool>(connection).unwrap(), true);

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200).unwrap();
        let query = select(sql::<Time>("'23:37:04.002200'").eq(roughly_half_past_eleven));
        assert_eq!(query.get_result::<bool>(connection).unwrap(), true);
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = NaiveTime::from_hms(0, 0, 0).unwrap();
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

        let noon = NaiveTime::from_hms(12, 0, 0).unwrap();
        let query = select(sql::<Time>("'12:00:00'"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200).unwrap();
        let query = select(sql::<Time>("'23:37:04.002200'"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(connection)
        );
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date!(2000 - 1 - 1);
        let query = select(date("2000-01-01").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = date!(0 - 4 - 11);
        let query = select(date("0000-04-11").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(date("2018-01-01").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = date!(9999 - 1 - 8);
        let query = select(date("9999-01-08").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date!(2000 - 1 - 1);
        let query = select(date("2000-01-01"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_past = date!(0 - 4 - 11);
        let query = select(date("0000-04-11"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(connection));

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(date("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_future = date!(9999 - 1 - 8);
        let query = select(date("9999-01-08"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDate>(connection)
        );
    }

    #[test]
    fn datetimes_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = datetime!(2000-1-1 1:1:1);
        let query = select(datetime("2000-01-01 01:01:01.000000"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<PrimitiveDateTime>(connection)
        );

        let distant_past = datetime!(0-4-11 2:2:2);
        let query = select(datetime("0000-04-11 02:02:02.000000"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<PrimitiveDateTime>(connection)
        );

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(date("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_future = datetime!(9999 - 1 - 8 23:59:59.0001);
        let query = select(sql::<Timestamp>("'9999-01-08 23:59:59.000100'"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<PrimitiveDateTime>(connection)
        );
    }

    #[test]
    fn datetimes_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = datetime!(2000-1-1 0:0:0);
        let query = select(datetime("2000-01-01 00:00:00.000000").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = datetime!(0-4-11 20:00:20);
        let query = select(datetime("0000-04-11 20:00:20.000000").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = datetime!(2018 - 1 - 1 12:00:00.0005);
        let query = select(sql::<Timestamp>("'2018-01-01 12:00:00.000500'").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = datetime!(9999-1-8 0:0:0);
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

        let time: OffsetDateTime = datetime!(1970-1-1 0:0:0.0 utc);

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
            .get_result::<OffsetDateTime>(conn)
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
                    .gt(datetime!(1970-1-1 0:0:0.0 utc)),
            )
            .filter(
                test_query_timestamptz_column_with_between::timestamp_with_tz
                    .lt(datetime!(1970-1-1 0:0:4.0 utc)),
            )
            .count()
            .get_result::<_>(conn);
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = &mut connection();
        // West one hour is negative offset
        let time = datetime!(1970-1-1 0:00:00.001 -1:00);
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 01:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = &mut connection();
        let time: OffsetDateTime = datetime!(1970-1-1 0:0:0.001 utc);
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 00:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly_with_utc_timezone_in_all_possible_formats() {
        let connection = &mut connection();
        let time: OffsetDateTime = datetime!(1970-1-1 0:0:0 utc);
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
