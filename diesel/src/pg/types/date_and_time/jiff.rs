//! This module makes it possible to map `jiff` date and time values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `jiff` feature.

use jiff::civil;
use jiff::{SignedDuration, Span, ZonedDateTime};

use super::{PgDate, PgInterval, PgTime, PgTimestamp};
use crate::deserialize::{self, Defaultable, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Interval, Time, Timestamp, Timestamptz};

// Postgres timestamps start from January 1st 2000.
const PG_EPOCH: civil::DateTime = civil::datetime(2000, 1, 1, 0, 0, 0, 0);

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Timestamp, Pg> for civil::DateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTimestamp(offset) = FromSql::<Timestamp, Pg>::from_sql(bytes)?;
        PG_EPOCH
            .checked_add(SignedDuration::from_micros(offset))
            .map_err(|_| "Tried to deserialize a timestamp that is too large for Jiff".into())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Timestamp, Pg> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let micros = self.duration_since(PG_EPOCH).as_micros();
        ToSql::<Timestamp, Pg>::to_sql(&PgTimestamp(micros.try_into()?), &mut out.reborrow())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for civil::DateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        FromSql::<Timestamp, Pg>::from_sql(bytes)
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Timestamptz, Pg> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        ToSql::<Timestamp, Pg>::to_sql(self, out)
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl Defaultable for civil::DateTime {
    fn default_value() -> Self {
        PG_EPOCH
    }
}

// Microseconds from the unix epoch (1970-01-01) to the postgres epoch (2000-01-01).
const PG_EPOCH_AS_UNIX_MICROS: i64 = 946_684_800_000_000;

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for jiff::Timestamp {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTimestamp(offset) = FromSql::<Timestamp, Pg>::from_sql(bytes)?;
        match offset.checked_add(PG_EPOCH_AS_UNIX_MICROS) {
            // Not `Timestamp::from_microsecond`, as that rejects the subsecond part of the last representable second
            Some(micros) => Ok(jiff::Timestamp::from_duration(
                SignedDuration::from_micros(micros),
            )?),
            None => {
                let message = "Tried to deserialize a timestamp that is too large for Jiff";
                Err(message.into())
            }
        }
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Timestamptz, Pg> for jiff::Timestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // jiff timestamps are limited to the years -9999 to 9999, so this cannot overflow
        let micros = self.as_microsecond() - PG_EPOCH_AS_UNIX_MICROS;
        ToSql::<Timestamp, Pg>::to_sql(&PgTimestamp(micros), &mut out.reborrow())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl Defaultable for jiff::Timestamp {
    fn default_value() -> Self {
        jiff::Timestamp::UNIX_EPOCH
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Timestamptz, Pg> for ZonedDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        ToSql::<Timestamptz, Pg>::to_sql(&self.timestamp(), &mut out.reborrow())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Time, Pg> for civil::Time {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let duration = self.duration_since(civil::Time::midnight());
        ToSql::<Time, Pg>::to_sql(
            &PgTime(duration.as_micros().try_into()?),
            &mut out.reborrow(),
        )
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Time, Pg> for civil::Time {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTime(offset) = FromSql::<Time, Pg>::from_sql(bytes)?;
        Ok(civil::Time::midnight().wrapping_add(SignedDuration::from_micros(offset)))
    }
}

const PG_EPOCH_DATE: civil::Date = civil::date(2000, 1, 1);

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Date, Pg> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let days_since_epoch = self.since(PG_EPOCH_DATE)?.get_days();
        ToSql::<Date, Pg>::to_sql(&PgDate(days_since_epoch), &mut out.reborrow())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Date, Pg> for civil::Date {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgDate(offset) = FromSql::<Date, Pg>::from_sql(bytes)?;
        Span::new()
            .try_days(i64::from(offset))
            .and_then(|days| PG_EPOCH_DATE.checked_add(days))
            .map_err(|_| {
                format!("Jiff can only represent dates up to {:?}", civil::Date::MAX).into()
            })
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl Defaultable for civil::Date {
    fn default_value() -> Self {
        PG_EPOCH_DATE
    }
}

const SECONDS_PER_DAY: i64 = 60 * 60 * 24;
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Interval, Pg> for Span {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let months: i32 =
            (i64::from(self.get_years()) * 12 + i64::from(self.get_months())).try_into()?;
        let days: i32 =
            (i64::from(self.get_weeks()) * 7 + i64::from(self.get_days())).try_into()?;
        // jiff limits every unit to at most 20 000 years worth of
        // that unit, so this sum cannot overflow
        let microseconds = i64::from(self.get_hours()) * 60 * 60 * MICROSECONDS_PER_SECOND
            + self.get_minutes() * 60 * MICROSECONDS_PER_SECOND
            + self.get_seconds() * MICROSECONDS_PER_SECOND
            + self.get_milliseconds() * 1_000
            + self.get_microseconds()
            + self.get_nanoseconds() / 1_000;
        let interval = PgInterval {
            microseconds,
            days,
            months,
        };
        <PgInterval as ToSql<Interval, Pg>>::to_sql(&interval, &mut out.reborrow())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Interval, Pg> for Span {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let interval: PgInterval = FromSql::<Interval, Pg>::from_sql(bytes)?;
        // jiff requires all units of a span to have the same sign and
        // silently flips the sign of already set units on mismatches
        let signums = [
            interval.months.signum(),
            interval.days.signum(),
            interval.microseconds.signum() as i32,
        ];
        if signums.contains(&1) && signums.contains(&-1) {
            return Err(format!(
                "Cannot represent a mixed sign interval as a jiff span: {interval:?}"
            )
            .into());
        }
        Ok(Span::new()
            .try_months(interval.months)?
            .try_days(interval.days)?
            .try_microseconds(interval.microseconds)?)
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl ToSql<Interval, Pg> for SignedDuration {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        const MICROSECONDS_PER_DAY: i128 = (SECONDS_PER_DAY * MICROSECONDS_PER_SECOND) as i128;
        let days: i32 = (self.as_micros() / MICROSECONDS_PER_DAY).try_into()?;
        #[allow(clippy::cast_possible_truncation)] // the remainder is smaller than a day
        let microseconds = (self.as_micros() % MICROSECONDS_PER_DAY) as i64;
        // We don't use months here, because in PostgreSQL
        // `timestamp - timestamp` returns interval where
        // every delta is contained in days and microseconds, and 0 months.
        // https://www.postgresql.org/docs/current/functions-datetime.html
        let interval = PgInterval {
            microseconds,
            days,
            months: 0,
        };
        <PgInterval as ToSql<Interval, Pg>>::to_sql(&interval, &mut out.reborrow())
    }
}

#[cfg(all(feature = "jiff", feature = "postgres_backend"))]
impl FromSql<Interval, Pg> for SignedDuration {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        const DAYS_PER_MONTH: i64 = 30;
        let interval: PgInterval = FromSql::<Interval, Pg>::from_sql(bytes)?;
        // We use 1 month = 30 days and 1 day = 24 hours, as postgres
        // use those ratios as default when explicitly converted.
        // For reference, please read `justify_interval` from this page.
        // https://www.postgresql.org/docs/current/functions-datetime.html
        let days = i64::from(interval.months) * DAYS_PER_MONTH + i64::from(interval.days);
        Ok(SignedDuration::from_secs(days * SECONDS_PER_DAY)
            + SignedDuration::from_micros(interval.microseconds))
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Date, Interval, Time, Timestamp, Timestamptz};
    use crate::test_helpers::connection;

    use jiff::civil::{self, date, datetime, time};
    use jiff::tz::{Offset, TimeZone};
    use jiff::{SignedDuration, Span};

    fn naive_now() -> civil::DateTime {
        Offset::UTC.to_datetime(jiff::Timestamp::now())
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = &mut connection();
        let time = jiff::Timestamp::UNIX_EPOCH;
        let query = select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = &mut connection();
        let time = Offset::constant(-1)
            .to_timestamp(datetime(1970, 1, 1, 0, 0, 0, 0))
            .unwrap();
        let query = select(sql::<Timestamptz>("'1970-01-01 01:00:00Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn zoned_encodes_correctly() {
        let connection = &mut connection();
        let time =
            jiff::Timestamp::UNIX_EPOCH.to_zoned_datetime(TimeZone::fixed(Offset::constant(-1)));
        let query = select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz").eq(time));
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
    fn unix_epoch_decodes_correctly_with_timezone() {
        let connection = &mut connection();
        let time = jiff::Timestamp::UNIX_EPOCH;
        let epoch_from_sql =
            select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[diesel_test_helper::test]
    fn maximal_timestamps_round_trip() {
        let connection = &mut connection();
        let time = jiff::Timestamp::new(jiff::Timestamp::MAX.as_second(), 999_999_000).unwrap();
        let query =
            select(sql::<Timestamptz>("'9999-12-30 22:00:00.999999Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
        let query = select(sql::<Timestamptz>(
            "'9999-12-30 22:00:00.999999Z'::timestamptz",
        ));
        assert_eq!(Ok(time), query.get_result::<jiff::Timestamp>(connection));
    }

    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = naive_now()
            .checked_add(SignedDuration::from_secs(60))
            .unwrap();
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = naive_now()
            .checked_sub(SignedDuration::from_secs(60))
            .unwrap();
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_with_timezones_round_trip_after_conversion() {
        let connection = &mut connection();
        let time = Offset::constant(1)
            .to_timestamp(datetime(2016, 1, 2, 1, 0, 0, 0))
            .unwrap();
        let expected = datetime(2016, 1, 1, 20, 0, 0, 0);
        let query = select(time.into_sql::<Timestamptz>().at_time_zone("EDT"));
        assert_eq!(Ok(expected), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let query = select(sql::<Time>("'00:00:00'::time").eq(civil::Time::midnight()));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("'12:00:00'::time").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.002200'::time").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let query = select(sql::<Time>("'00:00:00'::time"));
        assert_eq!(
            Ok(civil::Time::midnight()),
            query.get_result::<civil::Time>(connection)
        );

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("'12:00:00'::time"));
        assert_eq!(Ok(noon), query.get_result::<civil::Time>(connection));

        let roughly_half_past_eleven = time(23, 37, 4, 2_200_000);
        let query = select(sql::<Time>("'23:37:04.002200'::time"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<civil::Time>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = date(-398, 4, 11); // year 0 is 1 BC in this function
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
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<civil::Date>(connection)
        );

        let distant_past = date(-398, 4, 11);
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(
            Ok(distant_past),
            query.get_result::<civil::Date>(connection)
        );

        let julian_epoch = date(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(
            Ok(julian_epoch),
            query.get_result::<civil::Date>(connection)
        );

        let max_date = civil::Date::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result::<civil::Date>(connection));

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<civil::Date>(connection)
        );

        let distant_future = date(9999, 1, 8);
        let query = select(sql::<Date>("'9999-1-8'::date"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<civil::Date>(connection)
        );
    }

    /// Get test duration and corresponding literal SQL strings.
    fn get_test_duration_and_literal_strings() -> (SignedDuration, Vec<&'static str>) {
        (
            SignedDuration::from_hours(60 * 24)
                + SignedDuration::from_mins(1)
                + SignedDuration::from_micros(123456),
            vec![
                "60 days 1 minute 123456 microseconds",
                "2 months 1 minute 123456 microseconds",
                "5184060 seconds 123456 microseconds",
                "60 days 60123456 microseconds",
                "59 days 24 hours 60.123456 seconds",
                "60 0:01:00.123456",
                "58 48:01:00.123456",
                "P0Y2M0DT0H1M0.123456S",
                "0-2 0:01:00.123456",
                "P0000-02-00T00:01:00.123456",
                "1440:01:00.123456",
                "1 month 30 days 0.5 minutes 30.123456 seconds",
            ],
        )
    }

    #[diesel_test_helper::test]
    fn duration_encode_correctly() {
        let connection = &mut connection();
        let (duration, literal_strings) = get_test_duration_and_literal_strings();
        for literal in literal_strings {
            let query = select(sql::<Interval>(&format!("'{literal}'::interval")).eq(duration));
            assert!(query.get_result::<bool>(connection).unwrap());
        }
    }

    #[diesel_test_helper::test]
    fn duration_decode_correctly() {
        let connection = &mut connection();
        let (duration, literal_strings) = get_test_duration_and_literal_strings();
        for literal in literal_strings {
            let query = select(sql::<Interval>(&format!("'{literal}'::interval")));
            assert_eq!(Ok(duration), query.get_result::<SignedDuration>(connection));
        }
    }

    #[diesel_test_helper::test]
    fn span_encode_correctly() {
        let connection = &mut connection();
        let span = Span::new().months(2).minutes(1).microseconds(123_456);
        let query =
            select(sql::<Interval>("'2 months 1 minute 123456 microseconds'::interval").eq(span));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn span_decode_correctly() {
        let connection = &mut connection();
        let expected = Span::new().months(2).microseconds(60_123_456);
        let query = select(sql::<Interval>(
            "'2 months 1 minute 123456 microseconds'::interval",
        ));
        let span = query.get_result::<Span>(connection).unwrap();
        assert_eq!(expected.fieldwise(), span.fieldwise());
    }

    #[diesel_test_helper::test]
    fn mixed_sign_intervals_do_not_decode_to_spans() {
        let connection = &mut connection();
        let query = select(sql::<Interval>("'1 month -5 days'::interval"));
        assert!(query.get_result::<Span>(connection).is_err());
        let query = select(sql::<Interval>("'1 month -5 days'::interval"));
        assert_eq!(
            Ok(SignedDuration::from_hours(25 * 24)),
            query.get_result::<SignedDuration>(connection)
        );
    }
}
