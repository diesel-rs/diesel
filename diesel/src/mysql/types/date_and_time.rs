extern crate chrono;
extern crate mysqlclient_sys as ffi;

use self::chrono::*;
use std::error::Error;
use std::io::Write;
use std::os::raw as libc;
use std::{ptr, mem, slice};

use mysql::Mysql;
use types::{ToSql, FromSql, IsNull, Timestamp, Time, Date};

macro_rules! mysql_time_impls {
    ($ty:ty) => {
        impl ToSql<$ty, Mysql> for ffi::MYSQL_TIME {
            fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
                let bytes = unsafe {
                    let bytes_ptr = self as *const ffi::MYSQL_TIME as *const u8;
                    slice::from_raw_parts(bytes_ptr, mem::size_of::<ffi::MYSQL_TIME>())
                };
                out.write_all(bytes)?;
                Ok(IsNull::No)
            }
        }

        impl FromSql<$ty, Mysql> for ffi::MYSQL_TIME {
            fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
                let bytes = not_none!(bytes);
                let bytes_ptr = bytes.as_ptr() as *const ffi::MYSQL_TIME;
                unsafe {
                    let mut result = mem::uninitialized();
                    ptr::copy_nonoverlapping(bytes_ptr, &mut result, 1);
                    if result.neg == 0 {
                        Ok(result)
                    } else {
                        Err("Negative dates/times are not yet supported".into())
                    }
                }
            }
        }
    }
}

mysql_time_impls!(Timestamp);
mysql_time_impls!(Time);
mysql_time_impls!(Date);

impl ToSql<Timestamp, Mysql> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        let mut mysql_time: ffi::MYSQL_TIME = unsafe { mem::zeroed() };

        mysql_time.year = self.year() as libc::c_uint;
        mysql_time.month = self.month() as libc::c_uint;
        mysql_time.day = self.day() as libc::c_uint;
        mysql_time.hour = self.hour() as libc::c_uint;
        mysql_time.minute = self.minute() as libc::c_uint;
        mysql_time.second = self.second() as libc::c_uint;
        mysql_time.second_part = self.timestamp_subsec_micros() as libc::c_ulong;

        <ffi::MYSQL_TIME as ToSql<Timestamp, Mysql>>::to_sql(&mysql_time, out)
    }
}

impl FromSql<Timestamp, Mysql> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let mysql_time = <ffi::MYSQL_TIME as FromSql<Timestamp, Mysql>>::from_sql(bytes)?;
        Ok(NaiveDate::from_ymd(
            mysql_time.year as i32,
            mysql_time.month as u32,
            mysql_time.day as u32,
        ).and_hms_micro(
            mysql_time.hour as u32,
            mysql_time.minute as u32,
            mysql_time.second as u32,
            mysql_time.second_part as u32,
        ))
    }
}

impl ToSql<Time, Mysql> for NaiveTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        let mut mysql_time: ffi::MYSQL_TIME = unsafe { mem::zeroed() };

        mysql_time.hour = self.hour() as libc::c_uint;
        mysql_time.minute = self.minute() as libc::c_uint;
        mysql_time.second = self.second() as libc::c_uint;

        <ffi::MYSQL_TIME as ToSql<Time, Mysql>>::to_sql(&mysql_time, out)
    }
}

impl FromSql<Time, Mysql> for NaiveTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let mysql_time = <ffi::MYSQL_TIME as FromSql<Time, Mysql>>::from_sql(bytes)?;
        Ok(NaiveTime::from_hms(
            mysql_time.hour as u32,
            mysql_time.minute as u32,
            mysql_time.second as u32,
        ))
    }
}

impl ToSql<Date, Mysql> for NaiveDate {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        let mut mysql_time: ffi::MYSQL_TIME = unsafe { mem::zeroed() };

        mysql_time.year = self.year() as libc::c_uint;
        mysql_time.month = self.month() as libc::c_uint;
        mysql_time.day = self.day() as libc::c_uint;

        <ffi::MYSQL_TIME as ToSql<Date, Mysql>>::to_sql(&mysql_time, out)
    }
}

impl FromSql<Date, Mysql> for NaiveDate {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let mysql_time = <ffi::MYSQL_TIME as FromSql<Date, Mysql>>::from_sql(bytes)?;
        Ok(NaiveDate::from_ymd(
            mysql_time.year as i32,
            mysql_time.month as u32,
            mysql_time.day as u32,
        ))
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    extern crate dotenv;
    extern crate chrono;

    use self::chrono::{Duration, NaiveDate, NaiveTime, Utc, TimeZone, FixedOffset};
    use self::dotenv::dotenv;

    use expression::dsl::{sql, now};
    use mysql::MysqlConnection;
    use prelude::*;
    use select;
    use types::{Date, Time, Timestamp};

    fn connection() -> MysqlConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("MYSQL_UNIT_TEST_DATABASE_URL")
            .or_else(|_| ::std::env::var("MYSQL_DATABASE_URL"))
            .or_else(|_| ::std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        MysqlConnection::establish(&connection_url).unwrap()
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql = select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)"))
            .get_result(&connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = Utc::now().naive_utc() + Duration::days(1);
        let query = select(now.lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = Utc::now().naive_utc() - Duration::days(1);
        let query = select(now.gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = connection();

        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)").eq(midnight));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)").eq(noon));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms(23, 37, 4);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = connection();
        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)"));
        assert_eq!(Ok(midnight), query.get_result::<NaiveTime>(&connection));

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(&connection));

        let roughly_half_past_eleven = NaiveTime::from_hms(23, 37, 4);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)"));
        assert_eq!(Ok(roughly_half_past_eleven), query.get_result::<NaiveTime>(&connection));
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)").eq(january_first_2000));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)"));
        assert_eq!(Ok(january_first_2000), query.get_result::<NaiveDate>(&connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)"));
        assert_eq!(Ok(january_first_2018), query.get_result::<NaiveDate>(&connection));
    }
}
