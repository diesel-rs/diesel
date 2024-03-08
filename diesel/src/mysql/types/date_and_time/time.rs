use std::os::raw as libc;
use time::{
    Date as NaiveDate, Month, OffsetDateTime, PrimitiveDateTime, Time as NaiveTime, UtcOffset,
};

use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Datetime, Time, Timestamp};

use super::{MysqlTime, MysqlTimestampType};

fn to_time(dt: MysqlTime) -> Result<NaiveTime, Box<dyn std::error::Error>> {
    for (name, field) in [
        ("year", dt.year),
        ("month", dt.month),
        ("day", dt.day),
        ("offset", dt.time_zone_displacement as u32),
    ] {
        if field != 0 {
            return Err(format!("Unable to convert {dt:?} to time: {name} must be 0").into());
        }
    }

    let hour: u8 = dt.hour.try_into()?;
    let minute: u8 = dt.minute.try_into()?;
    let second: u8 = dt.second.try_into()?;
    let microsecond: u32 = dt.second_part.try_into()?;

    Ok(NaiveTime::from_hms_micro(
        hour,
        minute,
        second,
        microsecond,
    )?)
}

fn to_datetime(dt: MysqlTime) -> Result<OffsetDateTime, Box<dyn std::error::Error>> {
    let year: i32 = dt.year.try_into()?;
    let month: u8 = dt.month.try_into()?;
    let month: Month = month.try_into()?;
    let day: u8 = dt.day.try_into()?;
    let hour: u8 = dt.hour.try_into()?;
    let minute: u8 = dt.minute.try_into()?;
    let second: u8 = dt.second.try_into()?;
    let microsecond: u32 = dt.second_part.try_into()?;
    let offset = UtcOffset::from_whole_seconds(dt.time_zone_displacement)?;

    Ok(PrimitiveDateTime::new(
        NaiveDate::from_calendar_date(year, month, day)?,
        NaiveTime::from_hms_micro(hour, minute, second, microsecond)?,
    )
    .assume_offset(offset))
}

fn to_primitive_datetime(dt: OffsetDateTime) -> PrimitiveDateTime {
    let dt = dt.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(dt.date(), dt.time())
}

// Mysql datetime column has a wider range than timestamp column, so let's implement the fundamental operations in terms of datetime.
#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl ToSql<Datetime, Mysql> for PrimitiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year() as libc::c_uint,
            month: self.month() as libc::c_uint,
            day: self.day() as libc::c_uint,
            hour: self.hour() as libc::c_uint,
            minute: self.minute() as libc::c_uint,
            second: self.second() as libc::c_uint,
            second_part: libc::c_ulong::from(self.microsecond()),
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Timestamp, Mysql>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl FromSql<Datetime, Mysql> for PrimitiveDateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Timestamp, Mysql>>::from_sql(bytes)?;

        to_datetime(mysql_time)
            .map(to_primitive_datetime)
            .map_err(|err| format!("Cannot parse this date: {mysql_time:?}: {err}").into())
    }
}

// We can implement timestamps in terms of datetimes
#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl ToSql<Timestamp, Mysql> for PrimitiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        <PrimitiveDateTime as ToSql<Datetime, Mysql>>::to_sql(self, out)
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl FromSql<Timestamp, Mysql> for PrimitiveDateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        <PrimitiveDateTime as FromSql<Datetime, Mysql>>::from_sql(bytes)
    }
}

// Delegate offset datetimes in terms of UTC primitive datetimes; this stores everything in the DB as UTC
#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl ToSql<Datetime, Mysql> for OffsetDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        let prim = to_primitive_datetime(*self);
        <PrimitiveDateTime as ToSql<Datetime, Mysql>>::to_sql(&prim, &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl FromSql<Datetime, Mysql> for OffsetDateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let prim = <PrimitiveDateTime as FromSql<Datetime, Mysql>>::from_sql(bytes)?;
        Ok(prim.assume_offset(UtcOffset::UTC))
    }
}

// delegate timestamp column to datetime column for offset datetimes
#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl ToSql<Timestamp, Mysql> for OffsetDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        <OffsetDateTime as ToSql<Datetime, Mysql>>::to_sql(self, out)
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl FromSql<Timestamp, Mysql> for OffsetDateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        <OffsetDateTime as FromSql<Datetime, Mysql>>::from_sql(bytes)
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl ToSql<Time, Mysql> for NaiveTime {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Mysql>) -> serialize::Result {
        let mysql_time = MysqlTime {
            hour: self.hour() as libc::c_uint,
            minute: self.minute() as libc::c_uint,
            second: self.second() as libc::c_uint,
            day: 0,
            month: 0,
            second_part: 0,
            year: 0,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_TIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Time, Mysql>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl FromSql<Time, Mysql> for NaiveTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Time, Mysql>>::from_sql(bytes)?;

        to_time(mysql_time)
            .map_err(|err| format!("Unable to convert {mysql_time:?} to time: {err}").into())
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl ToSql<Date, Mysql> for NaiveDate {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year() as libc::c_uint,
            month: self.month() as libc::c_uint,
            day: self.day() as libc::c_uint,
            hour: 0,
            minute: 0,
            second: 0,
            second_part: 0,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATE,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Date, Mysql>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(all(feature = "time", feature = "mysql_backend"))]
impl FromSql<Date, Mysql> for NaiveDate {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Date, Mysql>>::from_sql(bytes)?;

        to_datetime(mysql_time)
            .map_err(|err| format!("Unable to convert {mysql_time:?} to time: {err}").into())
            .and_then(|dt| {
                let prim = to_primitive_datetime(dt);
                if prim.time() == NaiveTime::MIDNIGHT {
                    Ok(prim.date())
                } else {
                    Err(format!("Unable to convert {prim:?} to date: non-0 time part").into())
                }
            })
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;
    extern crate time;

    use time::{
        macros::{date, datetime, time},
        Date as NaiveDate, Duration, OffsetDateTime, Time as NaiveTime,
    };

    use super::to_primitive_datetime;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Date, Datetime, Time, Timestamp};
    use crate::test_helpers::connection;

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:0:0);
        let query = select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
        let query = select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = datetime!(1970-1-1 0:0:0);
        let epoch_from_sql =
            select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
        let epoch_from_sql =
            select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let time = to_primitive_datetime(OffsetDateTime::now_utc()) + Duration::days(1);
        let query = select(now.lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = to_primitive_datetime(OffsetDateTime::now_utc()) - Duration::days(1);
        let query = select(now.gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = time!(0:0:0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = time!(12:0:0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = time!(23:37:4);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = time!(0:0:0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)"));
        assert_eq!(Ok(midnight), query.get_result::<NaiveTime>(connection));

        let noon = time!(12:0:0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(connection));

        let roughly_half_past_eleven = time!(23:37:4);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(connection)
        );
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date!(2000 - 1 - 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date!(2000 - 1 - 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(connection)
        );

        let january_first_2018 = date!(2018 - 1 - 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)"));
        assert_eq!(
            Ok(january_first_2018),
            query.get_result::<NaiveDate>(connection)
        );

        crate::sql_query("SET sql_mode = (SELECT REPLACE(@@sql_mode, 'NO_ZERO_DATE,', ''))")
            .execute(connection)
            .unwrap();
        let query = select(sql::<Date>("CAST('0000-00-00' AS DATE)"));
        assert!(query.get_result::<NaiveDate>(connection).is_err());
    }
}
