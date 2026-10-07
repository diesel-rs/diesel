//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.

extern crate chrono;
use self::chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

use super::{PgDate, PgInterval, PgTime, PgTimestamp};
use crate::deserialize::{self, Defaultable, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Interval, Time, Timestamp, Timestamptz};

// Postgres timestamps start from January 1st 2000.
fn pg_epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 1)
        .expect("This is in supported range of chrono dates")
        .and_hms_opt(0, 0, 0)
        .expect("This is a valid input")
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Timestamp, Pg> for NaiveDateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
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

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl ToSql<Timestamp, Pg> for NaiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let time = match (self.signed_duration_since(pg_epoch())).num_microseconds() {
            Some(time) => time,
            None => {
                let error_message =
                    format!("{self:?} as microseconds is too large to fit in an i64");
                return Err(error_message.into());
            }
        };
        ToSql::<Timestamp, Pg>::to_sql(&PgTimestamp(time), &mut out.reborrow())
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for NaiveDateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        FromSql::<Timestamp, Pg>::from_sql(bytes)
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl ToSql<Timestamptz, Pg> for NaiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        ToSql::<Timestamp, Pg>::to_sql(self, out)
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl Defaultable for NaiveDateTime {
    fn default_value() -> Self {
        Self::default()
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for DateTime<Utc> {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let naive_date_time = <NaiveDateTime as FromSql<Timestamptz, Pg>>::from_sql(bytes)?;
        Ok(Utc.from_utc_datetime(&naive_date_time))
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl Defaultable for DateTime<Utc> {
    fn default_value() -> Self {
        Self::default()
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for DateTime<Local> {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let naive_date_time = <NaiveDateTime as FromSql<Timestamptz, Pg>>::from_sql(bytes)?;
        Ok(Local::from_utc_datetime(&Local, &naive_date_time))
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl Defaultable for DateTime<Local> {
    fn default_value() -> Self {
        Self::default()
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl<TZ: TimeZone> ToSql<Timestamptz, Pg> for DateTime<TZ> {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        ToSql::<Timestamptz, Pg>::to_sql(&self.naive_utc(), &mut out.reborrow())
    }
}

fn midnight() -> NaiveTime {
    NaiveTime::from_hms_opt(0, 0, 0).expect("This is a valid hms spec")
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl ToSql<Time, Pg> for NaiveTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let duration = self.signed_duration_since(midnight());
        match duration.num_microseconds() {
            Some(offset) => ToSql::<Time, Pg>::to_sql(&PgTime(offset), &mut out.reborrow()),
            None => unreachable!(),
        }
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Time, Pg> for NaiveTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTime(offset) = FromSql::<Time, Pg>::from_sql(bytes)?;
        let duration = Duration::microseconds(offset);
        Ok(midnight() + duration)
    }
}

fn pg_epoch_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2000, 1, 1).expect("This is in supported range of chrono dates")
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl ToSql<Date, Pg> for NaiveDate {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let days_since_epoch = self.signed_duration_since(pg_epoch_date()).num_days();
        ToSql::<Date, Pg>::to_sql(&PgDate(days_since_epoch.try_into()?), &mut out.reborrow())
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Date, Pg> for NaiveDate {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgDate(offset) = FromSql::<Date, Pg>::from_sql(bytes)?;
        #[allow(deprecated)] // otherwise we would need to bump our minimal chrono version
        let duration = Duration::days(i64::from(offset));
        match pg_epoch_date().checked_add_signed(duration) {
            Some(date) => Ok(date),
            None => {
                let error_message = format!(
                    "Chrono can only represent dates up to {:?}",
                    chrono::NaiveDate::MAX
                );
                Err(error_message.into())
            }
        }
    }
}

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl Defaultable for NaiveDate {
    fn default_value() -> Self {
        Self::default()
    }
}

const DAYS_PER_MONTH: i64 = 30;
const SECONDS_PER_DAY: i64 = 60 * 60 * 24;
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl ToSql<Interval, Pg> for Duration {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let microseconds: i64 = if let Some(v) = self.num_microseconds() {
            v % (MICROSECONDS_PER_SECOND * SECONDS_PER_DAY)
        } else {
            return Err("Failed to create microseconds by overflow".into());
        };
        let days: i32 = self
            .num_days()
            .try_into()
            .expect("Failed to get i32 days from i64");
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

#[cfg(all(feature = "chrono", feature = "postgres_backend"))]
impl FromSql<Interval, Pg> for Duration {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let interval: PgInterval = FromSql::<Interval, Pg>::from_sql(bytes)?;
        // We use 1 month = 30 days and 1 day = 24 hours, as postgres
        // use those ratios as default when explicitly converted.
        // For reference, please read `justify_interval` from this page.
        // https://www.postgresql.org/docs/current/functions-datetime.html
        // widened, since any `i32` month and day pair fits `i64` days and chrono
        let days = i64::from(interval.months) * DAYS_PER_MONTH + i64::from(interval.days);
        Ok(Duration::days(days) + Duration::microseconds(interval.microseconds))
    }
}

#[cfg(test)]
mod tests {
    extern crate chrono;
    extern crate dotenvy;

    use self::chrono::{Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Date, Interval, Time, Timestamp, Timestamptz};
    use crate::test_helpers::connection;

    #[diesel_test_helper::test]
    fn regression_interval_months_do_not_overflow() {
        use crate::pg::value::PgValue;

        let mut offenders = Vec::new();
        // 71_582_788 months is where the product leaves `i32`
        for months in [
            0,
            1,
            -1,
            71_582_787,
            71_582_788,
            71_582_789,
            -71_582_789,
            i32::MAX,
            i32::MIN,
        ] {
            for days in [0, 1, -1, i32::MAX, i32::MIN] {
                for microseconds in [0, 1, -1, i64::MAX, i64::MIN] {
                    let mut buffer = Vec::new();
                    buffer.extend_from_slice(&microseconds.to_be_bytes());
                    buffer.extend_from_slice(&days.to_be_bytes());
                    buffer.extend_from_slice(&months.to_be_bytes());

                    let expected = Duration::days(i64::from(months) * 30 + i64::from(days))
                        + Duration::microseconds(microseconds);
                    let read = <Duration as crate::deserialize::FromSql<Interval, crate::pg::Pg>>::from_sql(
                        PgValue::for_test(&buffer),
                    );
                    if read.as_ref().ok() != Some(&expected) {
                        offenders.push(format!(
                            "{months} months {days} days {microseconds} us read as {read:?}, not {expected:?}"
                        ));
                    }
                }
            }
        }
        // a deterministic xorshift over the whole three field space
        let mut state: u64 = 0x2545F4914F6CDD1D;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..4096 {
            let bytes = next().to_ne_bytes();
            let months = i32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let days = i32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            let microseconds = i64::from_ne_bytes(next().to_ne_bytes());
            let mut buffer = Vec::new();
            buffer.extend_from_slice(&microseconds.to_be_bytes());
            buffer.extend_from_slice(&days.to_be_bytes());
            buffer.extend_from_slice(&months.to_be_bytes());

            let expected = Duration::days(i64::from(months) * 30 + i64::from(days))
                + Duration::microseconds(microseconds);
            let read = <Duration as crate::deserialize::FromSql<Interval, crate::pg::Pg>>::from_sql(
                PgValue::for_test(&buffer),
            );
            if read.as_ref().ok() != Some(&expected) {
                offenders.push(format!(
                    "{months} months {days} days {microseconds} us read as {read:?}, not {expected:?}"
                ));
            }
        }

        assert!(
            offenders.is_empty(),
            "{} intervals converted wrongly, first {:?}",
            offenders.len(),
            &offenders[..offenders.len().min(3)]
        );
    }

    #[diesel_test_helper::test]
    fn regression_interval_from_postgres_does_not_overflow() {
        let connection = &mut connection();
        // postgres stores each of these happily, so none is a synthetic blob
        for (literal, months, days) in [
            ("178956970 years 7 mons", i32::MAX, 0),
            ("1 mon 2147483647 days", 1, i32::MAX),
            ("-178956970 years -8 mons", i32::MIN, 0),
        ] {
            let read = select(sql::<Interval>(&format!("'{literal}'::interval")))
                .get_result::<Duration>(connection)
                .unwrap();
            let expected = Duration::days(i64::from(months) * 30 + i64::from(days));
            assert_eq!(read, expected, "{literal} read as {read:?}");
        }
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = &mut connection();
        let time = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).single().unwrap();
        let query = select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = &mut connection();
        let time = FixedOffset::west_opt(3600)
            .unwrap()
            .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
            .single()
            .unwrap();
        let query = select(sql::<Timestamptz>("'1970-01-01 01:00:00Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let epoch_from_sql =
            select(sql::<Timestamp>("'1970-01-01'::timestamp")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly_with_timezone() {
        let connection = &mut connection();
        let time = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).single().unwrap();
        let epoch_from_sql =
            select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = Utc::now().naive_utc() + Duration::try_seconds(60).unwrap();
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = Utc::now().naive_utc() - Duration::try_seconds(60).unwrap();
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_with_timezones_round_trip_after_conversion() {
        let connection = &mut connection();
        let time = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2016, 1, 2, 1, 0, 0)
            .unwrap();
        let expected = NaiveDate::from_ymd_opt(2016, 1, 1)
            .unwrap()
            .and_hms_opt(20, 0, 0)
            .unwrap();
        let query = select(time.into_sql::<Timestamptz>().at_time_zone("EDT"));
        assert_eq!(Ok(expected), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let query = select(sql::<Time>("'00:00:00'::time").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let query = select(sql::<Time>("'12:00:00'::time").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms_micro_opt(23, 37, 4, 2200).unwrap();
        let query = select(sql::<Time>("'23:37:04.002200'::time").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let query = select(sql::<Time>("'00:00:00'::time"));
        assert_eq!(Ok(midnight), query.get_result::<NaiveTime>(connection));

        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let query = select(sql::<Time>("'12:00:00'::time"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_micro_opt(23, 37, 4, 2200).unwrap();
        let query = select(sql::<Time>("'23:37:04.002200'::time"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let query = select(sql::<Date>("'2000-1-1'").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = NaiveDate::from_ymd_opt(-398, 4, 11).unwrap(); // year 0 is 1 BC in this function
        let query = select(sql::<Date>("'399-4-11 BC'").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let julian_epoch = NaiveDate::from_ymd_opt(-4713, 11, 24).unwrap();
        let query = select(sql::<Date>("'J0'::date").eq(julian_epoch));
        assert!(query.get_result::<bool>(connection).unwrap());

        let max_date = NaiveDate::from_ymd_opt(262142, 12, 31).unwrap();
        let query = select(sql::<Date>("'262142-12-31'::date").eq(max_date));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
        let query = select(sql::<Date>("'2018-1-1'::date").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = NaiveDate::from_ymd_opt(72_400, 1, 8).unwrap();
        let query = select(sql::<Date>("'72400-1-8'::date").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let query = select(sql::<Date>("'2000-1-1'::date"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_past = NaiveDate::from_ymd_opt(-398, 4, 11).unwrap();
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(connection));

        let julian_epoch = NaiveDate::from_ymd_opt(-4713, 11, 24).unwrap();
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(Ok(julian_epoch), query.get_result::<NaiveDate>(connection));

        let max_date = NaiveDate::from_ymd_opt(262142, 12, 31).unwrap();
        let query = select(sql::<Date>("'262142-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result::<NaiveDate>(connection));

        let january_first_2018 = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_future = NaiveDate::from_ymd_opt(72_400, 1, 8).unwrap();
        let query = select(sql::<Date>("'72400-1-8'::date"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDate>(connection)
        );
    }

    /// Get test duration and corresponding literal SQL strings.
    fn get_test_duration_and_literal_strings() -> (Duration, Vec<&'static str>) {
        (
            Duration::days(60) + Duration::minutes(1) + Duration::microseconds(123456),
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
            assert_eq!(Ok(duration), query.get_result::<Duration>(connection));
        }
    }
}
