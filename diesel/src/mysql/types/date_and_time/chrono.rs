use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::os::raw as libc;

use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Datetime, Time, Timestamp};

use super::{MysqlTime, MysqlTimestampType};

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
impl ToSql<Datetime, Mysql> for NaiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        <NaiveDateTime as ToSql<Timestamp, Mysql>>::to_sql(self, out)
    }
}

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
impl FromSql<Datetime, Mysql> for NaiveDateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        <NaiveDateTime as FromSql<Timestamp, Mysql>>::from_sql(bytes)
    }
}

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
impl ToSql<Timestamp, Mysql> for NaiveDateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year() as libc::c_uint,
            month: self.month() as libc::c_uint,
            day: self.day() as libc::c_uint,
            hour: self.hour() as libc::c_uint,
            minute: self.minute() as libc::c_uint,
            second: self.second() as libc::c_uint,
            #[allow(deprecated)] // otherwise we would need to bump our minimal chrono version
            second_part: libc::c_ulong::from(self.timestamp_subsec_micros()),
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Timestamp, Mysql>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
impl FromSql<Timestamp, Mysql> for NaiveDateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Timestamp, Mysql>>::from_sql(bytes)?;

        NaiveDate::from_ymd_opt(mysql_time.year as i32, mysql_time.month, mysql_time.day)
            .and_then(|v| {
                v.and_hms_micro_opt(
                    mysql_time.hour,
                    mysql_time.minute,
                    mysql_time.second,
                    mysql_time.second_part as u32,
                )
            })
            .ok_or_else(|| format!("Cannot parse this date: {mysql_time:?}").into())
    }
}

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
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

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
impl FromSql<Time, Mysql> for NaiveTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Time, Mysql>>::from_sql(bytes)?;
        NaiveTime::from_hms_opt(mysql_time.hour, mysql_time.minute, mysql_time.second)
            .ok_or_else(|| format!("Unable to convert {mysql_time:?} to chrono").into())
    }
}

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
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

#[cfg(all(feature = "chrono", feature = "mysql_backend"))]
impl FromSql<Date, Mysql> for NaiveDate {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Date, Mysql>>::from_sql(bytes)?;
        NaiveDate::from_ymd_opt(mysql_time.year as i32, mysql_time.month, mysql_time.day)
            .ok_or_else(|| format!("Unable to convert {mysql_time:?} to chrono").into())
    }
}

#[cfg(test)]
mod tests {
    extern crate chrono;
    extern crate dotenvy;

    use self::chrono::{Duration, NaiveDate, NaiveTime, Utc};

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::{Date, Datetime, Time, Timestamp};
    use crate::test_helpers::connection;

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let query = select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
        let query = select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
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
        let time = Utc::now().naive_utc() + Duration::try_days(1).unwrap();
        let query = select(now.lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = Utc::now().naive_utc() - Duration::try_days(1).unwrap();
        let query = select(now.gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();

        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms_opt(23, 37, 4).unwrap();
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)"));
        assert_eq!(Ok(midnight), query.get_result::<NaiveTime>(connection));

        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_opt(23, 37, 4).unwrap();
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)"));
        assert_eq!(
            Ok(roughly_half_past_eleven),
            query.get_result::<NaiveTime>(connection)
        );
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)"));
        assert_eq!(
            Ok(january_first_2000),
            query.get_result::<NaiveDate>(connection)
        );

        let january_first_2018 = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
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
