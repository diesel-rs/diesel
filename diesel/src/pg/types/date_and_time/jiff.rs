//! This module makes it possible to map `jiff` date and time values to postgres `Date`,
//! `Time` and `Timestamp` fields. It is enabled with the `jiff` feature.

use jiff::civil;
use jiff::{SignedDuration, Span};

use super::{PgDate, PgTime, PgTimestamp};
use crate::deserialize::{self, Defaultable, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Time, Timestamp};

// Postgres dates and timestamps start from January 1st 2000.
const PG_EPOCH_DATE: civil::Date = civil::date(2000, 1, 1);
const PG_EPOCH: civil::DateTime = PG_EPOCH_DATE.at(0, 0, 0, 0);

impl FromSql<Timestamp, Pg> for civil::DateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTimestamp(micros_since_epoch) = FromSql::<Timestamp, Pg>::from_sql(bytes)?;
        let duration = SignedDuration::from_micros(micros_since_epoch);
        Ok(PG_EPOCH.checked_add(duration)?)
    }
}

impl ToSql<Timestamp, Pg> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let micros_since_epoch = self.duration_since(PG_EPOCH).as_micros();
        ToSql::<Timestamp, Pg>::to_sql(
            &PgTimestamp(micros_since_epoch.try_into()?),
            &mut out.reborrow(),
        )
    }
}

impl Defaultable for civil::DateTime {
    fn default_value() -> Self {
        PG_EPOCH
    }
}

impl ToSql<Time, Pg> for civil::Time {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let micros_since_midnight = self.duration_since(civil::Time::midnight()).as_micros();
        ToSql::<Time, Pg>::to_sql(
            &PgTime(micros_since_midnight.try_into()?),
            &mut out.reborrow(),
        )
    }
}

impl FromSql<Time, Pg> for civil::Time {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTime(micros_since_midnight) = FromSql::<Time, Pg>::from_sql(bytes)?;
        let duration = SignedDuration::from_micros(micros_since_midnight);
        // `24:00:00` wraps around to midnight, like in the `chrono` and `time` integrations.
        Ok(civil::Time::midnight().wrapping_add(duration))
    }
}

impl ToSql<Date, Pg> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let days_since_epoch = self.since(PG_EPOCH_DATE)?.get_days();
        ToSql::<Date, Pg>::to_sql(&PgDate(days_since_epoch), &mut out.reborrow())
    }
}

impl FromSql<Date, Pg> for civil::Date {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgDate(days_since_epoch) = FromSql::<Date, Pg>::from_sql(bytes)?;
        let span = Span::new().try_days(days_since_epoch)?;
        Ok(PG_EPOCH_DATE.checked_add(span)?)
    }
}

impl Defaultable for civil::Date {
    fn default_value() -> Self {
        PG_EPOCH_DATE
    }
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;
    use jiff::civil::{self, date, datetime, time};
    use jiff::tz::Offset;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::result::Error::{DatabaseError, DeserializationError};
    use crate::select;
    use crate::sql_types::{Date, Time, Timestamp};
    use crate::test_helpers::connection;

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let epoch_from_sql =
            select(sql::<Timestamp>("'1970-01-01'::timestamp")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let naive_now = Offset::UTC.to_datetime(jiff::Timestamp::now());
        let time = naive_now + SignedDuration::from_secs(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = naive_now - SignedDuration::from_secs(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn datetimes_round_trip() {
        let connection = &mut connection();
        for (value, text) in [
            (datetime(2000, 1, 1, 0, 0, 0, 0), "2000-01-01 00:00:00"),
            (
                datetime(2018, 1, 1, 12, 0, 0, 500_000),
                "2018-01-01 12:00:00.0005",
            ),
            (
                datetime(-4713, 11, 24, 0, 0, 0, 0),
                "4714-11-24 00:00:00 BC",
            ),
            (
                datetime(9999, 12, 31, 23, 59, 59, 999_999_000),
                "9999-12-31 23:59:59.999999",
            ),
        ] {
            let query = select(sql::<Timestamp>(&format!("'{text}'::timestamp")).eq(value));
            assert!(query.get_result::<bool>(connection).unwrap(), "{text}");

            let query = select(sql::<Timestamp>(&format!("'{text}'::timestamp")));
            assert_eq!(Ok(value), query.get_result(connection), "{text}");
        }
    }

    #[diesel_test_helper::test]
    fn datetimes_are_truncated_to_microseconds() {
        let connection = &mut connection();
        let query =
            select(sql::<Timestamp>("'9999-12-31 23:59:59.999999'").eq(civil::DateTime::MAX));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn datetimes_outside_of_jiff_range_fail_to_decode() {
        let connection = &mut connection();
        for text in ["10000-01-01 00:00:00", "infinity", "-infinity"] {
            let query = select(sql::<Timestamp>(&format!("'{text}'::timestamp")));
            assert!(
                matches!(
                    query.get_result::<civil::DateTime>(connection),
                    Err(DeserializationError(_))
                ),
                "{text}"
            );
        }
    }

    #[diesel_test_helper::test]
    fn datetimes_before_postgres_range_fail_to_encode() {
        let connection = &mut connection();
        let query = select(civil::DateTime::MIN.into_sql::<Timestamp>());
        assert!(matches!(
            query.get_result::<civil::DateTime>(connection),
            Err(DatabaseError(..))
        ));
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();
        let midnight = time(0, 0, 0, 0);
        let query = select(sql::<Time>("'00:00:00'::time").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("'12:00:00'::time").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.002200'::time").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());

        let max_time = civil::Time::MAX;
        let query = select(sql::<Time>("'23:59:59.999999'::time").eq(max_time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = time(0, 0, 0, 0);
        let query = select(sql::<Time>("'00:00:00'::time"));
        assert_eq!(Ok(midnight), query.get_result(connection));

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("'12:00:00'::time"));
        assert_eq!(Ok(noon), query.get_result(connection));

        let roughly_half_past_eleven = time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.002200'::time"));
        assert_eq!(Ok(roughly_half_past_eleven), query.get_result(connection));

        let last_microsecond = time(23, 59, 59, 999_999_000);
        let query = select(sql::<Time>("'23:59:59.999999'::time"));
        assert_eq!(Ok(last_microsecond), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn end_of_day_decodes_as_midnight() {
        let connection = &mut connection();
        let query = select(sql::<Time>("'24:00:00'::time"));
        assert_eq!(Ok(time(0, 0, 0, 0)), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = date(-398, 4, 11); // jiff uses year 0 for 1 BC
        let query = select(sql::<Date>("'399-4-11 BC'").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let julian_epoch = date(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date").eq(julian_epoch));
        assert!(query.get_result::<bool>(connection).unwrap());

        let max_date = civil::Date::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date").eq(max_date));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = date(9999, 1, 8);
        let query = select(sql::<Date>("'9999-1-8'::date").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'::date"));
        assert_eq!(Ok(january_first_2000), query.get_result(connection));

        let distant_past = date(-398, 4, 11);
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(Ok(distant_past), query.get_result(connection));

        let julian_epoch = date(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(Ok(julian_epoch), query.get_result(connection));

        let max_date = civil::Date::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result(connection));

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(Ok(january_first_2018), query.get_result(connection));

        let distant_future = date(9999, 1, 8);
        let query = select(sql::<Date>("'9999-1-8'::date"));
        assert_eq!(Ok(distant_future), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn dates_outside_of_jiff_range_fail_to_decode() {
        let connection = &mut connection();
        let query = select(sql::<Date>("'10000-1-1'::date"));
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DeserializationError(_))
        ));

        let query = select(sql::<Date>("'infinity'::date"));
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DeserializationError(_))
        ));

        let query = select(sql::<Date>("'-infinity'::date"));
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DeserializationError(_))
        ));
    }

    #[diesel_test_helper::test]
    fn dates_before_postgres_range_fail_to_encode() {
        let connection = &mut connection();
        let query = select(civil::Date::MIN.into_sql::<Date>());
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DatabaseError(..))
        ));
    }
}
