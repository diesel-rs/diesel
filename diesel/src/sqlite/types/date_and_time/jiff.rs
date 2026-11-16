//! This module makes it possible to map `jiff` date and time values to sqlite `NUMERIC`
//! fields. It is enabled with the `jiff` feature.

use alloc::format;
use alloc::string::ToString;

use jiff::{SignedDuration, civil};

use crate::backend::Backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Date, Time, Timestamp};
use crate::sqlite::Sqlite;

const DATE_FORMAT: &str = "%F";

const ENCODE_TIME_FORMAT: &str = "%T%.f";

// `%.f` is optional when parsing. SQLite shifts values with a `+HH:MM` or
// `-HH:MM` suffix to UTC, so ignoring the suffix would yield a different time.
// `Z` needs no shift and is the only suffix accepted.
const TIME_FORMATS: [&str; 4] = [
    "%T%.f",  // 23:37:04, 23:37:04.0022
    "%R",     // 23:37
    "%RZ",    // 23:37Z
    "%T%.fZ", // 23:37:04Z, 23:37:04.0022Z
];

const ENCODE_DATETIME_FORMAT: &str = "%F %T%.f";

// Offsets other than `Z` are rejected for the same reason as in `TIME_FORMATS`.
const DATETIME_FORMATS: [&str; 8] = [
    "%F %T%.f",  // 2000-01-01 23:37:04, 2000-01-01 23:37:04.0022
    "%F %R",     // 2000-01-01 23:37
    "%F %RZ",    // 2000-01-01 23:37Z
    "%F %T%.fZ", // 2000-01-01 23:37:04Z, 2000-01-01 23:37:04.0022Z
    "%FT%T%.f",  // 2000-01-01T23:37:04, 2000-01-01T23:37:04.0022
    "%FT%R",     // 2000-01-01T23:37
    "%FT%RZ",    // 2000-01-01T23:37Z
    "%FT%T%.fZ", // 2000-01-01T23:37:04Z, 2000-01-01T23:37:04.0022Z
];

const UNIX_EPOCH: civil::DateTime = civil::datetime(1970, 1, 1, 0, 0, 0, 0);

// SQLite also accepts Julian day numbers like `2440587.5` as datetimes.
fn parse_julian_day(text: &str) -> Option<civil::DateTime> {
    const UNIX_EPOCH_IN_JULIAN_DAYS: f64 = 2_440_587.5;
    const SECONDS_PER_DAY: f64 = 86_400.0;
    let julian_days = text.parse::<f64>().ok()?;
    let seconds_since_epoch = (julian_days - UNIX_EPOCH_IN_JULIAN_DAYS) * SECONDS_PER_DAY;
    let duration = SignedDuration::try_from_secs_f64(seconds_since_epoch).ok()?;
    UNIX_EPOCH.checked_add(duration).ok()
}

impl FromSql<Date, Sqlite> for civil::Date {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        value
            .parse_string(|s| civil::Date::strptime(DATE_FORMAT, s))
            .map_err(Into::into)
    }
}

impl ToSql<Date, Sqlite> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.strftime(DATE_FORMAT).to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Time, Sqlite> for civil::Time {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            TIME_FORMATS
                .into_iter()
                .find_map(|format| civil::Time::strptime(format, text).ok())
                .ok_or_else(|| format!("Invalid time {text}").into())
        })
    }
}

impl ToSql<Time, Sqlite> for civil::Time {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.strftime(ENCODE_TIME_FORMAT).to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Timestamp, Sqlite> for civil::DateTime {
    fn from_sql(mut value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        value.parse_string(|text| {
            DATETIME_FORMATS
                .into_iter()
                .find_map(|format| civil::DateTime::strptime(format, text).ok())
                .or_else(|| parse_julian_day(text))
                .ok_or_else(|| format!("Invalid datetime {text}").into())
        })
    }
}

impl ToSql<Timestamp, Sqlite> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.strftime(ENCODE_DATETIME_FORMAT).to_string());
        Ok(IsNull::No)
    }
}

#[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
mod tests {
    use jiff::civil;
    // `SignedDuration`, `Offset` and `now` are only used by the test that
    // compares against the current time, which Miri cannot read from the host
    // while isolation is enabled.
    #[cfg(not(miri))]
    use jiff::{SignedDuration, tz::Offset};

    #[cfg(not(miri))]
    use crate::dsl::now;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::result::Error::DeserializationError;
    use crate::select;
    use crate::sql_types::{Date, Text, Time, Timestamp};
    use crate::test_helpers::connection;

    #[declare_sql_function]
    extern "SQL" {
        fn date(x: Text) -> Date;
        fn time(x: Text) -> Time;
        fn datetime(x: Text) -> Timestamp;
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = civil::datetime(1970, 1, 1, 0, 0, 0, 0);
        let query = select(datetime("1970-01-01 00:00:00.000000").eq(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly_in_all_possible_formats() {
        let connection = &mut connection();
        let time = civil::datetime(1970, 1, 1, 0, 0, 0, 0);
        for text in [
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
            "2440587.5",
        ] {
            let query = select(sql::<Timestamp>(&format!("'{text}'")));
            assert_eq!(
                Ok(time),
                query.get_result(connection),
                "format {text} failed"
            );
        }
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn datetimes_with_offsets_fail_to_decode() {
        let connection = &mut connection();
        for text in [
            "1970-01-01 00:00+01:00",
            "1970-01-01 00:00:00+00:00",
            "1970-01-01T00:00:00.000-01:00",
        ] {
            let query = select(sql::<Timestamp>(&format!("'{text}'")));
            assert!(
                matches!(
                    query.get_result::<civil::DateTime>(connection),
                    Err(DeserializationError(_))
                ),
                "format {text} was accepted"
            );
        }
    }

    // This test compares against the current time, which Miri cannot read
    // from the host while isolation is enabled.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let naive_now = Offset::UTC.to_datetime(jiff::Timestamp::now());
        let time = naive_now + SignedDuration::from_secs(60);
        let query = select(now.lt(time));
        assert_eq!(Ok(true), query.get_result(connection));

        let time = naive_now - SignedDuration::from_secs(600);
        let query = select(now.gt(time));
        assert_eq!(Ok(true), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();
        let midnight = civil::time(0, 0, 0, 0);
        let query = select(time("00:00:00").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = civil::time(12, 0, 0, 0);
        let query = select(time("12:00:00").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = civil::time(23, 37, 4, 2_200_000);
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
        let midnight = civil::time(0, 0, 0, 0);
        for text in [
            "00:00",
            "00:00:00",
            "00:00:00.000",
            "00:00:00.000000",
            "00:00Z",
            "00:00:00Z",
            "00:00:00.000Z",
            "00:00:00.000000Z",
        ] {
            let query = select(sql::<Time>(&format!("'{text}'")));
            assert_eq!(
                Ok(midnight),
                query.get_result(connection),
                "format {text} failed"
            );
        }

        let roughly_half_past_eleven = civil::time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.0022'"));
        assert_eq!(Ok(roughly_half_past_eleven), query.get_result(connection));
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn times_with_offsets_fail_to_decode() {
        let connection = &mut connection();
        for text in ["00:00+01:00", "00:00:00+00:00", "00:00:00.000-05:00"] {
            let query = select(sql::<Time>(&format!("'{text}'")));
            assert!(
                matches!(
                    query.get_result::<civil::Time>(connection),
                    Err(DeserializationError(_))
                ),
                "format {text} was accepted"
            );
        }
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn times_round_trip_through_text() {
        let connection = &mut connection();
        for (value, text) in [
            (civil::time(0, 0, 0, 0), "00:00:00"),
            (civil::time(23, 37, 4, 2_200_000), "23:37:04.0022"),
            (civil::Time::MAX, "23:59:59.999999999"),
        ] {
            let query = select(value.into_sql::<Time>());
            assert_eq!(Ok(text.to_owned()), query.get_result::<String>(connection));

            let query = select(sql::<Time>(&format!("'{text}'")));
            assert_eq!(Ok(value), query.get_result::<civil::Time>(connection));
        }
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = civil::date(2000, 1, 1);
        let query = select(date("2000-01-01").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = civil::date(0, 4, 11);
        let query = select(date("0000-04-11").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = civil::date(2018, 1, 1);
        let query = select(date("2018-01-01").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = civil::date(9999, 1, 8);
        let query = select(date("9999-01-08").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = civil::date(2000, 1, 1);
        let query = select(date("2000-01-01"));
        assert_eq!(Ok(january_first_2000), query.get_result(connection));

        let distant_past = civil::date(0, 4, 11);
        let query = select(date("0000-04-11"));
        assert_eq!(Ok(distant_past), query.get_result(connection));

        let january_first_2018 = civil::date(2018, 1, 1);
        let query = select(date("2018-01-01"));
        assert_eq!(Ok(january_first_2018), query.get_result(connection));

        let distant_future = civil::date(9999, 1, 8);
        let query = select(date("9999-01-08"));
        assert_eq!(Ok(distant_future), query.get_result(connection));
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn dates_round_trip_through_text() {
        let connection = &mut connection();
        for (value, text) in [
            (civil::date(2000, 1, 1), "2000-01-01"),
            (civil::date(0, 4, 11), "0000-04-11"),
            (civil::Date::MAX, "9999-12-31"),
        ] {
            let query = select(value.into_sql::<Date>());
            assert_eq!(Ok(text.to_owned()), query.get_result::<String>(connection));

            let query = select(sql::<Date>(&format!("'{text}'")));
            assert_eq!(Ok(value), query.get_result::<civil::Date>(connection));
        }
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn datetimes_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = civil::datetime(2000, 1, 1, 1, 1, 1, 0);
        let query = select(datetime("2000-01-01 01:01:01.000000"));
        assert_eq!(Ok(january_first_2000), query.get_result(connection));

        let distant_past = civil::datetime(0, 4, 11, 2, 2, 2, 0);
        let query = select(datetime("0000-04-11 02:02:02.000000"));
        assert_eq!(Ok(distant_past), query.get_result(connection));

        let january_first_2018 = civil::date(2018, 1, 1);
        let query = select(date("2018-01-01"));
        assert_eq!(Ok(january_first_2018), query.get_result(connection));

        let distant_future = civil::datetime(9999, 1, 8, 23, 59, 59, 100_000);
        let query = select(sql::<Timestamp>("'9999-01-08 23:59:59.000100'"));
        assert_eq!(Ok(distant_future), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn datetimes_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = civil::datetime(2000, 1, 1, 0, 0, 0, 0);
        let query = select(datetime("2000-01-01 00:00:00").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = civil::datetime(0, 4, 11, 20, 0, 20, 0);
        let query = select(datetime("0000-04-11 20:00:20").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = civil::datetime(2018, 1, 1, 12, 0, 0, 500_000);
        let query = select(sql::<Timestamp>("'2018-01-01 12:00:00.0005'").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = civil::datetime(9999, 1, 8, 0, 0, 0, 0);
        let query = select(datetime("9999-01-08 00:00:00").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    // This test decodes query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn datetimes_round_trip_through_text() {
        let connection = &mut connection();
        for (value, text) in [
            (
                civil::datetime(2000, 1, 1, 1, 1, 1, 0),
                "2000-01-01 01:01:01",
            ),
            (
                civil::datetime(9999, 1, 8, 23, 59, 59, 100_000),
                "9999-01-08 23:59:59.0001",
            ),
            (civil::DateTime::MAX, "9999-12-31 23:59:59.999999999"),
        ] {
            let query = select(value.into_sql::<Timestamp>());
            assert_eq!(Ok(text.to_owned()), query.get_result::<String>(connection));

            let query = select(sql::<Timestamp>(&format!("'{text}'")));
            assert_eq!(Ok(value), query.get_result::<civil::DateTime>(connection));
        }
    }
}
