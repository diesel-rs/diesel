//! This module makes it possible to map `jiff` date and time values to sqlite `NUMERIC`
//! fields. It is enabled with the `jiff` feature.

use alloc::format;
use alloc::string::ToString;

use jiff::civil;

use crate::backend::Backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Date, Time};
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

#[cfg(all(test, not(all(target_family = "wasm", target_os = "unknown"))))]
mod tests {
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::result::Error::DeserializationError;
    use crate::select;
    use crate::sql_types::{Date, Text, Time};
    use crate::test_helpers::connection;
    use jiff::civil;

    #[declare_sql_function]
    extern "SQL" {
        fn date(x: Text) -> Date;
        fn time(x: Text) -> Time;
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
}
