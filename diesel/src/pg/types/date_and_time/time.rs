//! This module makes it possible to map `time` date and time values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `time` feature.

extern crate time;

use self::time::{
    macros::{date, datetime},
    Date as NaiveDate, Duration, OffsetDateTime, PrimitiveDateTime, Time as NaiveTime, UtcOffset,
};

use super::{PgDate, PgTime, PgTimestamp};
use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Time, Timestamp, Timestamptz};

// Postgres timestamps start from January 1st 2000.
const PG_EPOCH: PrimitiveDateTime = datetime!(2000-1-1 0:00:00);

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl FromSql<Timestamp, Pg> for PrimitiveDateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTimestamp(offset) = FromSql::<Timestamp, Pg>::from_sql(bytes)?;
        match PG_EPOCH.checked_add(Duration::microseconds(offset)) {
            Some(v) => Ok(v),
            None => {
                let message = "Tried to deserialize a timestamp that is too large for Time";
                Err(message.into())
            }
        }
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl ToSql<Timestamp, Pg> for PrimitiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let micros = (*self - PG_EPOCH).whole_microseconds();
        if micros > (i64::MAX as i128) {
            let error_message = format!("{self:?} as microseconds is too large to fit in an i64");
            return Err(error_message.into());
        }
        let micros = micros as i64;
        ToSql::<Timestamp, Pg>::to_sql(&PgTimestamp(micros), &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for PrimitiveDateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        FromSql::<Timestamp, Pg>::from_sql(bytes)
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl ToSql<Timestamptz, Pg> for PrimitiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        ToSql::<Timestamp, Pg>::to_sql(self, out)
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl FromSql<Timestamptz, Pg> for OffsetDateTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let primitive_date_time = <PrimitiveDateTime as FromSql<Timestamptz, Pg>>::from_sql(bytes)?;
        Ok(primitive_date_time.assume_utc())
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl ToSql<Timestamptz, Pg> for OffsetDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let as_utc = self.to_offset(UtcOffset::UTC);
        let primitive_date_time = PrimitiveDateTime::new(as_utc.date(), as_utc.time());
        ToSql::<Timestamptz, Pg>::to_sql(&primitive_date_time, &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl ToSql<Time, Pg> for NaiveTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let duration = *self - NaiveTime::MIDNIGHT;
        // microseconds in a day cannot overflow i64
        let micros = duration.whole_microseconds() as i64;
        ToSql::<Time, Pg>::to_sql(&PgTime(micros), &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl FromSql<Time, Pg> for NaiveTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgTime(offset) = FromSql::<Time, Pg>::from_sql(bytes)?;
        let duration = Duration::microseconds(offset);
        Ok(NaiveTime::MIDNIGHT + duration)
    }
}

const PG_EPOCH_DATE: NaiveDate = date!(2000 - 1 - 1);

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl ToSql<Date, Pg> for NaiveDate {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let days_since_epoch = (*self - PG_EPOCH_DATE).whole_days();
        ToSql::<Date, Pg>::to_sql(&PgDate(days_since_epoch as i32), &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "postgres_backend"))]
impl FromSql<Date, Pg> for NaiveDate {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgDate(offset) = FromSql::<Date, Pg>::from_sql(bytes)?;
        match PG_EPOCH_DATE.checked_add(Duration::days(i64::from(offset))) {
            Some(date) => Ok(date),
            None => {
                let error_message =
                    format!("Time can only represent dates up to {:?}", NaiveDate::MAX);
                Err(error_message.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Date, Time, Timestamp, Timestamptz};
    use crate::test_helpers::connection;

    use time::{
        macros::{date, datetime},
        Date as NaiveDate, Duration, OffsetDateTime, PrimitiveDateTime, Time as NaiveTime,
    };

    fn naive_now() -> PrimitiveDateTime {
        let offset_now = OffsetDateTime::now_utc();
        PrimitiveDateTime::new(offset_now.date(), offset_now.time())
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:00:00);
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_utc_timezone() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:00:00 utc);
        let query = select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_encodes_correctly_with_timezone() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:00:00 -1:00);
        let query = select(sql::<Timestamptz>("'1970-01-01 01:00:00Z'::timestamptz").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:0:0);
        let epoch_from_sql =
            select(sql::<Timestamp>("'1970-01-01'::timestamp")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn unix_epoch_decodes_correctly_with_timezone() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:00:00 utc);
        let epoch_from_sql =
            select(sql::<Timestamptz>("'1970-01-01Z'::timestamptz")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = naive_now() + Duration::seconds(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = naive_now() - Duration::seconds(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_with_timezones_round_trip_after_conversion() {
        let connection = &mut connection();
        let time = datetime!(2016-1-2 1:00:00 +1);
        let expected = datetime!(2016-1-1 20:0:0);
        let query = select(time.into_sql::<Timestamptz>().at_time_zone("EDT"));
        assert_eq!(Ok(expected), query.get_result(connection));
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let query = select(sql::<Time>("'00:00:00'::time").eq(NaiveTime::MIDNIGHT));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = NaiveTime::from_hms(12, 0, 0).expect("noon is a legal time");
        let query = select(sql::<Time>("'12:00:00'::time").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven =
            NaiveTime::from_hms_micro(23, 37, 4, 2200).expect("this is also a legal time");
        let query = select(sql::<Time>("'23:37:04.002200'::time").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let query = select(sql::<Time>("'00:00:00'::time"));
        let result: Result<NaiveTime, _> = query.get_result(connection);
        assert_eq!(Ok(NaiveTime::MIDNIGHT), result);

        let noon = NaiveTime::from_hms(12, 0, 0).expect("this time is legal");
        let query = select(sql::<Time>("'12:00:00'::time"));
        let result: Result<NaiveTime, _> = query.get_result(connection);
        assert_eq!(Ok(noon), result);

        let roughly_half_past_eleven =
            NaiveTime::from_hms_micro(23, 37, 4, 2200).expect("this time is legal");
        let query = select(sql::<Time>("'23:37:04.002200'::time"));
        let result: Result<NaiveTime, _> = query.get_result(connection);
        assert_eq!(Ok(roughly_half_past_eleven), result);
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date!(2000 - 1 - 1);
        let query = select(sql::<Date>("'2000-1-1'").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = date!(-398 - 4 - 11); // year 0 is 1 BC in this function
        let query = select(sql::<Date>("'399-4-11 BC'").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let julian_epoch = date!(-4713 - 11 - 24);
        let query = select(sql::<Date>("'J0'::date").eq(julian_epoch));
        assert!(query.get_result::<bool>(connection).unwrap());

        let max_date = NaiveDate::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date").eq(max_date));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(sql::<Date>("'2018-1-1'::date").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = date!(9999 - 1 - 8);
        let query = select(sql::<Date>("'9999-1-8'::date").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date!(2000 - 1 - 1);
        let query = select(sql::<Date>("'2000-1-1'::date"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_past = date!(-398 - 4 - 11);
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(connection));

        let julian_epoch = date!(-4713 - 11 - 24);
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(Ok(julian_epoch), query.get_result::<NaiveDate>(connection));

        let max_date = NaiveDate::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result::<NaiveDate>(connection));

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        let distant_future = date!(9999 - 1 - 8);
        let query = select(sql::<Date>("'9999-1-8'::date"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<NaiveDate>(connection)
        );
    }
}
