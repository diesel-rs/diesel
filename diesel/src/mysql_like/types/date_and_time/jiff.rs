use jiff::ZonedDateTime;
use jiff::civil;
use jiff::tz::Offset;
use std::os::raw as libc;

use crate::deserialize::{self, FromSql};
use crate::mysql_like::{MysqlLikeBackend, MysqlValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Datetime, Time, Timestamp};

use super::{MysqlTime, MysqlTimestampType};

fn to_time(dt: MysqlTime) -> Result<civil::Time, Box<dyn core::error::Error>> {
    for (name, field) in [
        ("year", dt.year),
        ("month", dt.month),
        ("day", dt.day),
        ("offset", dt.time_zone_displacement.try_into()?),
    ] {
        if field != 0 {
            return Err(format!("Unable to convert {dt:?} to time: {name} must be 0").into());
        }
    }

    let hour: i8 = dt.hour.try_into()?;
    let minute: i8 = dt.minute.try_into()?;
    let second: i8 = dt.second.try_into()?;
    let microsecond: i32 = dt.second_part.try_into()?;

    Ok(civil::Time::new(
        hour,
        minute,
        second,
        microsecond.saturating_mul(1_000),
    )?)
}

fn to_datetime(dt: MysqlTime) -> Result<civil::DateTime, Box<dyn core::error::Error>> {
    let year: i16 = dt.year.try_into()?;
    let month: i8 = dt.month.try_into()?;
    let day: i8 = dt.day.try_into()?;
    let hour: i8 = dt.hour.try_into()?;
    let minute: i8 = dt.minute.try_into()?;
    let second: i8 = dt.second.try_into()?;
    let microsecond: i32 = dt.second_part.try_into()?;

    let datetime = civil::DateTime::new(
        year,
        month,
        day,
        hour,
        minute,
        second,
        microsecond.saturating_mul(1_000),
    )?;

    match dt.time_zone_displacement {
        // Skip the round trip through a timestamp below, as jiff timestamps
        // cannot represent the last day of the mysql datetime range
        0 => Ok(datetime),
        displacement => {
            let offset = Offset::from_seconds(displacement)?;
            Ok(to_civil_datetime(offset.to_timestamp(datetime)?))
        }
    }
}

fn to_timestamp(dt: MysqlTime) -> Result<jiff::Timestamp, Box<dyn core::error::Error>> {
    Ok(Offset::UTC.to_timestamp(to_datetime(dt)?)?)
}

fn to_civil_datetime(ts: jiff::Timestamp) -> civil::DateTime {
    Offset::UTC.to_datetime(ts)
}

// Mysql datetime column has a wider range than timestamp column, so let's implement the fundamental operations in terms of datetime.
#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Datetime, DB> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year().try_into()?,
            month: self.month().try_into()?,
            day: self.day().try_into()?,
            hour: self.hour().try_into()?,
            minute: self.minute().try_into()?,
            second: self.second().try_into()?,
            second_part: libc::c_ulong::try_from(self.subsec_nanosecond() / 1_000)?,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Timestamp, DB>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> FromSql<Datetime, DB> for civil::DateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Timestamp, DB>>::from_sql(bytes)?;

        to_datetime(mysql_time)
            .map_err(|err| format!("Cannot parse this date: {mysql_time:?}: {err}").into())
    }
}

// We can implement timestamps in terms of datetimes
#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Timestamp, DB> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        <civil::DateTime as ToSql<Datetime, DB>>::to_sql(self, out)
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> FromSql<Timestamp, DB> for civil::DateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        <civil::DateTime as FromSql<Datetime, DB>>::from_sql(bytes)
    }
}

// Delegate timestamps in terms of UTC civil datetimes; this stores everything in the DB as UTC
#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Datetime, DB> for jiff::Timestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let datetime = to_civil_datetime(*self);
        <civil::DateTime as ToSql<Datetime, DB>>::to_sql(&datetime, &mut out.reborrow())
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> FromSql<Datetime, DB> for jiff::Timestamp {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Timestamp, DB>>::from_sql(bytes)?;

        to_timestamp(mysql_time)
            .map_err(|err| format!("Cannot parse this date: {mysql_time:?}: {err}").into())
    }
}

// delegate timestamp column to datetime column for timestamps
#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Timestamp, DB> for jiff::Timestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        <jiff::Timestamp as ToSql<Datetime, DB>>::to_sql(self, out)
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> FromSql<Timestamp, DB> for jiff::Timestamp {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        <jiff::Timestamp as FromSql<Datetime, DB>>::from_sql(bytes)
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Datetime, DB> for ZonedDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        ToSql::<Datetime, DB>::to_sql(&self.timestamp(), &mut out.reborrow())
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Timestamp, DB> for ZonedDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        ToSql::<Datetime, DB>::to_sql(self, out)
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Time, DB> for civil::Time {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
        let mysql_time = MysqlTime {
            hour: self.hour().try_into()?,
            minute: self.minute().try_into()?,
            second: self.second().try_into()?,
            day: 0,
            month: 0,
            second_part: libc::c_ulong::try_from(self.subsec_nanosecond() / 1_000)?,
            year: 0,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_TIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Time, DB>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> FromSql<Time, DB> for civil::Time {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Time, DB>>::from_sql(bytes)?;

        to_time(mysql_time)
            .map_err(|err| format!("Unable to convert {mysql_time:?} to time: {err}").into())
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> ToSql<Date, DB> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year().try_into()?,
            month: self.month().try_into()?,
            day: self.day().try_into()?,
            hour: 0,
            minute: 0,
            second: 0,
            second_part: 0,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATE,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Date, DB>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(feature = "jiff")]
impl<DB: MysqlLikeBackend> FromSql<Date, DB> for civil::Date {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Date, DB>>::from_sql(bytes)?;

        to_datetime(mysql_time)
            .map_err(|err| format!("Unable to convert {mysql_time:?} to jiff: {err}").into())
            .and_then(|datetime| {
                if datetime.time() == civil::Time::midnight() {
                    Ok(datetime.date())
                } else {
                    Err(format!("Unable to convert {datetime:?} to date: non-0 time part").into())
                }
            })
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use jiff::SignedDuration;
    use jiff::civil::{date, datetime, time};

    use super::to_civil_datetime;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Date, Datetime, Time, Timestamp};
    use crate::test_helpers::connection;

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let query = select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
        let query = select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = jiff::Timestamp::UNIX_EPOCH;
        let query = select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let epoch_from_sql =
            select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
        let epoch_from_sql =
            select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);

        let time = jiff::Timestamp::UNIX_EPOCH;
        let epoch_from_sql =
            select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = to_civil_datetime(jiff::Timestamp::now())
            .checked_add(SignedDuration::from_hours(24))
            .unwrap();
        let query = select(now.lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = to_civil_datetime(jiff::Timestamp::now())
            .checked_sub(SignedDuration::from_hours(24))
            .unwrap();
        let query = select(now.gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = time(0, 0, 0, 0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = time(23, 37, 4, 0);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = time(0, 0, 0, 0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)"));
        assert_eq!(
            Ok(midnight),
            query.get_result::<jiff::civil::Time>(connection)
        );

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)"));
        assert_eq!(Ok(noon), query.get_result::<jiff::civil::Time>(connection));

        let roughly_half_past_eleven = time(23, 37, 4, 0);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<jiff::civil::Time>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn times_of_day_with_microseconds_encode_correctly() {
        let connection = &mut connection();

        let with_micros = time(8, 30, 0, 123_456_000);
        let query = select(sql::<Time>("CAST('08:30:00.123456' AS TIME(6))").eq(with_micros));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_with_microseconds_decode_correctly() {
        let connection = &mut connection();

        let with_micros = time(8, 30, 0, 123_456_000);
        let query = select(sql::<Time>("CAST('08:30:00.123456' AS TIME(6))"));
        assert_eq!(
            Ok(with_micros),
            query.get_result::<jiff::civil::Time>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn maximal_datetimes_round_trip() {
        let connection = &mut connection();
        let time = datetime(9999, 12, 31, 23, 59, 59, 0);
        let query = select(sql::<Datetime>("CAST('9999-12-31 23:59:59' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
        let query = select(sql::<Datetime>("CAST('9999-12-31 23:59:59' AS DATETIME)"));
        assert_eq!(
            Ok(time),
            query.get_result::<jiff::civil::DateTime>(connection)
        );
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = date(9999, 12, 31);
        let query = select(sql::<Date>("CAST('9999-12-31' AS DATE)").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<jiff::civil::Date>(connection)
        );

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<jiff::civil::Date>(connection)
        );

        let distant_future = date(9999, 12, 31);
        let query = select(sql::<Date>("CAST('9999-12-31' AS DATE)"));
        assert_eq!(
            Ok(distant_future),
            query.get_result::<jiff::civil::Date>(connection)
        );

        crate::sql_query("SET sql_mode = (SELECT REPLACE(@@sql_mode, 'NO_ZERO_DATE,', ''))")
            .execute(connection)
            .unwrap();
        let query = select(sql::<Date>("CAST('0000-00-00' AS DATE)"));
        assert!(query.get_result::<jiff::civil::Date>(connection).is_err());
    }
}
