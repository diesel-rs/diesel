//! This module makes it possible to map `jiff` date and time values to sqlite `NUMERIC`
//! fields. It is enabled with the `jiff` feature.

use jiff::ZonedDateTime;
use jiff::civil;
use jiff::fmt::strtime;
use jiff::tz::Offset;

use crate::backend::Backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Date, Time, Timestamp, TimestamptzSqlite};
use crate::sqlite::Sqlite;

/// Warning to future editors:
/// Changes in the following formats need to be kept in sync
/// with the formats of the ["chrono"](super::chrono) and
/// ["time"](super::time) modules.
/// We do not need a distinction between whole second and
/// subsecond since %.f will only print the dot if needed.
/// We print as few subsecond digits as possible,
/// this means the subsecond part can be between 1 and 9 digits.
const DATE_FORMAT: &str = "%F";

const ENCODE_TIME_FORMAT: &str = "%T%.f";

const TIME_FORMATS: [&str; 9] = [
    // Most likely formats
    "%T%.f", "%T", // All other valid formats in order of increasing specificity
    "%R", "%RZ", "%R%:z", "%TZ", "%T%:z", "%T%.fZ", "%T%.f%:z",
];

const ENCODE_NAIVE_DATETIME_FORMAT: &str = "%F %T%.f";

const ENCODE_DATETIME_FORMAT: &str = "%F %T%.f%:z";

const NAIVE_DATETIME_FORMATS: [&str; 18] = [
    // Most likely formats
    "%F %T%.f",
    "%F %T%.f%:z",
    "%F %T",
    "%F %T%:z",
    // All other formats in order of increasing specificity
    "%F %R",
    "%F %RZ",
    "%F %R%:z",
    "%F %TZ",
    "%F %T%.fZ",
    "%FT%R",
    "%FT%RZ",
    "%FT%R%:z",
    "%FT%T",
    "%FT%TZ",
    "%FT%T%:z",
    "%FT%T%.f",
    "%FT%T%.fZ",
    "%FT%T%.f%:z",
];

const DATETIME_FORMATS: [&str; 12] = [
    // Most likely formats
    "%F %T%.f%:z",
    "%F %T%:z",
    // All other formats in order of increasing specificity
    "%F %RZ",
    "%F %R%:z",
    "%F %TZ",
    "%F %T%.fZ",
    "%FT%RZ",
    "%FT%R%:z",
    "%FT%TZ",
    "%FT%T%:z",
    "%FT%T%.fZ",
    "%FT%T%.f%:z",
];

fn parse_julian(julian_days: f64) -> Result<civil::DateTime, jiff::Error> {
    const EPOCH_IN_JULIAN_DAYS: f64 = 2_440_587.5;
    const SECONDS_IN_DAY: f64 = 86400.0;
    let timestamp = (julian_days - EPOCH_IN_JULIAN_DAYS) * SECONDS_IN_DAY;
    #[allow(clippy::cast_possible_truncation)] // we multiply by 1E9 to prevent that
    jiff::Timestamp::from_nanosecond((timestamp * 1E9) as i128)
        .map(|timestamp| Offset::UTC.to_datetime(timestamp))
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl FromSql<Date, Sqlite> for civil::Date {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        value
            .parse_string(|s| Self::strptime(DATE_FORMAT, s))
            .map_err(Into::into)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl ToSql<Date, Sqlite> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(strtime::format(DATE_FORMAT, *self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl FromSql<Time, Sqlite> for civil::Time {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            for format in TIME_FORMATS {
                if let Ok(time) = Self::strptime(format, text) {
                    return Ok(time);
                }
            }

            Err(format!("Invalid time {text}").into())
        })
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl ToSql<Time, Sqlite> for civil::Time {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(strtime::format(ENCODE_TIME_FORMAT, *self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl FromSql<Timestamp, Sqlite> for civil::DateTime {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            for format in NAIVE_DATETIME_FORMATS {
                if let Ok(dt) = Self::strptime(format, text) {
                    return Ok(dt);
                }
            }

            if let Ok(julian_days) = text.parse::<f64>()
                && let Ok(timestamp) = parse_julian(julian_days)
            {
                return Ok(timestamp);
            }

            Err(format!("Invalid datetime {text}").into())
        })
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl ToSql<Timestamp, Sqlite> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(strtime::format(ENCODE_NAIVE_DATETIME_FORMAT, *self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl FromSql<TimestamptzSqlite, Sqlite> for civil::DateTime {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <Self as FromSql<Timestamp, Sqlite>>::from_sql(value)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl ToSql<TimestamptzSqlite, Sqlite> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(strtime::format(ENCODE_NAIVE_DATETIME_FORMAT, *self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl FromSql<TimestamptzSqlite, Sqlite> for jiff::Timestamp {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        // First try to parse the timezone
        if let Ok(timestamp) = value.parse_string(|text| {
            for format in DATETIME_FORMATS {
                if let Ok(timestamp) = Self::strptime(format, text) {
                    return Ok(timestamp);
                }
            }

            Err(())
        }) {
            return Ok(timestamp);
        }

        // Fallback on assuming UTC
        let datetime = <civil::DateTime as FromSql<TimestamptzSqlite, Sqlite>>::from_sql(value)?;
        Ok(Offset::UTC.to_timestamp(datetime)?)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl ToSql<TimestamptzSqlite, Sqlite> for jiff::Timestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(strtime::format(ENCODE_DATETIME_FORMAT, *self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "jiff"))]
impl ToSql<TimestamptzSqlite, Sqlite> for ZonedDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        // Converting to UTC ensures consistency
        out.set_value(strtime::format(ENCODE_DATETIME_FORMAT, self.timestamp())?);
        Ok(IsNull::No)
    }
}

#[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
#[allow(clippy::unwrap_used)]
mod tests {
    extern crate dotenvy;

    #[cfg(not(miri))]
    use jiff::SignedDuration;
    use jiff::civil::{date, datetime, time};
    use jiff::tz::{Offset, TimeZone};

    #[cfg(not(miri))]
    use crate::dsl::now;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Text, Time, Timestamp, TimestamptzSqlite};
    use crate::test_helpers::connection;

    // these functions carry a `_sql` suffix to avoid a name
    // clash with the constructor functions from `jiff::civil`
    #[declare_sql_function]
    extern "SQL" {
        #[sql_name = "datetime"]
        fn datetime_sql(x: Text) -> Timestamp;
        #[sql_name = "time"]
        fn time_sql(x: Text) -> Time;
        #[sql_name = "date"]
        fn date_sql(x: Text) -> Date;
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let query = select(datetime_sql("1970-01-01 00:00:00.000000").eq(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly_in_all_possible_formats() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
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
            let epoch_from_sql = select(sql::<Timestamp>(&format!("'{s}'"))).get_result(connection);
            assert_eq!(Ok(time), epoch_from_sql, "format {s} failed");
        }
    }

    // Miri cannot read the host clock while isolation is enabled.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = Offset::UTC
            .to_datetime(jiff::Timestamp::now())
            .checked_add(SignedDuration::from_secs(60))
            .unwrap();
        let query = select(now.lt(time));
        assert_eq!(Ok(true), query.get_result(connection));

        let time = Offset::UTC
            .to_datetime(jiff::Timestamp::now())
            .checked_sub(SignedDuration::from_secs(600))
            .unwrap();
        let query = select(now.gt(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = time(0, 0, 0, 0);
        let query = select(time_sql("00:00:00").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = time(12, 0, 0, 0);
        let query = select(time_sql("12:00:00").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.0022'").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = time(0, 0, 0, 0);
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
            let query = select(sql::<Time>(&format!("'{format}'")));
            assert_eq!(
                Ok(midnight),
                query.get_result::<jiff::civil::Time>(connection),
                "format {format} failed"
            );
        }

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("'12:00:00'"));
        assert_eq!(Ok(noon), query.get_result::<jiff::civil::Time>(connection));

        let roughly_half_past_eleven = time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.002200'"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<jiff::civil::Time>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(date_sql("2000-01-01").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = date(0, 4, 11);
        let query = select(date_sql("0000-04-11").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date(2018, 1, 1);
        let query = select(date_sql("2018-01-01").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = date(9999, 1, 8);
        let query = select(date_sql("9999-01-08").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(date_sql("2000-01-01"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<jiff::civil::Date>(connection)
        );

        let distant_past = date(0, 4, 11);
        let query = select(date_sql("0000-04-11"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<jiff::civil::Date>(connection)
        );

        let january_first_2018 = date(2018, 1, 1);
        let query = select(date_sql("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<jiff::civil::Date>(connection)
        );

        let distant_future = date(9999, 1, 8);
        let query = select(date_sql("9999-01-08"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<jiff::civil::Date>(connection)
        );
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn datetimes_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = datetime(2000, 1, 1, 1, 1, 1, 0);
        let query = select(datetime_sql("2000-01-01 01:01:01.000000"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<jiff::civil::DateTime>(connection)
        );

        let distant_past = datetime(0, 4, 11, 2, 2, 2, 0);
        let query = select(datetime_sql("0000-04-11 02:02:02.000000"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<jiff::civil::DateTime>(connection)
        );

        let january_first_2018 = date(2018, 1, 1);
        let query = select(date_sql("2018-01-01"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<jiff::civil::Date>(connection)
        );

        let distant_future = datetime(9999, 1, 8, 23, 59, 59, 100_000);
        let query = select(sql::<Timestamp>("'9999-01-08 23:59:59.000100'"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<jiff::civil::DateTime>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn datetimes_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = datetime(2000, 1, 1, 0, 0, 0, 0);
        let query = select(datetime_sql("2000-01-01 00:00:00").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = datetime(0, 4, 11, 20, 0, 20, 0);
        let query = select(datetime_sql("0000-04-11 20:00:20").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = datetime(2018, 1, 1, 12, 0, 0, 500_000);
        let query = select(sql::<Timestamp>("'2018-01-01 12:00:00.0005'").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = datetime(9999, 1, 8, 0, 0, 0, 0);
        let query = select(datetime_sql("9999-01-08 00:00:00").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
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

        let time = jiff::Timestamp::UNIX_EPOCH;

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
            .get_result::<jiff::Timestamp>(conn)
            .unwrap();
        assert_eq!(result, time);
    }

    #[diesel_test_helper::test]
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
                    .gt(jiff::Timestamp::UNIX_EPOCH),
            )
            .filter(
                test_query_timestamptz_column_with_between::timestamp_with_tz
                    .lt(jiff::Timestamp::from_second(4).unwrap()),
            )
            .count()
            .get_result::<_>(conn);
        assert_eq!(result, Ok(3));
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = &mut connection();
        // West one hour is negative offset
        let time = Offset::constant(-1)
            .to_timestamp(datetime(1970, 1, 1, 0, 0, 0, 1_000_000))
            .unwrap();
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 01:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = datetime(1970, 1, 1, 0, 0, 0, 1_000_000)
            .to_zoned_datetime(TimeZone::fixed(Offset::constant(-1)))
            .unwrap();
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 01:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = &mut connection();
        let time = jiff::Timestamp::from_millisecond(1).unwrap();
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 00:00:00.001+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        // and without millisecond
        let time = jiff::Timestamp::UNIX_EPOCH;
        let query = select(sql::<TimestamptzSqlite>("'1970-01-01 00:00:00+00:00'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly_with_utc_timezone_in_all_possible_formats() {
        let connection = &mut connection();
        let time = jiff::Timestamp::UNIX_EPOCH;
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
            "2440587.5",
        ];

        for s in valid_epoch_formats {
            let epoch_from_sql =
                select(sql::<TimestamptzSqlite>(&format!("'{s}'"))).get_result(connection);
            assert_eq!(Ok(time), epoch_from_sql, "format {s} failed");
        }
    }
}
