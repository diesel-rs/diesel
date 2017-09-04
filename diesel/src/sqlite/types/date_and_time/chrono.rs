extern crate chrono;

use std::error::Error;
use std::io::Write;
use self::chrono::{NaiveDateTime, NaiveDate, NaiveTime, Datelike};

use sqlite::Sqlite;
use sqlite::connection::SqliteValue;
use types::{Date, FromSql, IsNull, Time, Timestamp, ToSql, ToSqlOutput, Text};

const SQLITE_TIME_FMT: &'static str = "%0H:%0M:%0S%.6f";

fn parse_sqlite_date(date: &str) -> Result<NaiveDate, Box<Error+Send+Sync>> {
    let negative_year = date.starts_with('-');

    let ymd: Vec<u32> = date.split('-').flat_map(str::parse).collect();
    if ymd.len() != 3 {
        return Err("Cannot parse date, too many parts. The date should be in the form: YYYY-MM-DD".into())
    }

    if negative_year {
        return Ok(NaiveDate::from_ymd(-(ymd[0] as i32), ymd[1], ymd[2]))
    }

    match NaiveDate::from_ymd_opt(ymd[0] as i32, ymd[1], ymd[2]) {
        Some(d) => Ok(d),
        None => Err("Invalid date.".into()),
    }
}

fn dump_sqlite_date<T: Datelike>(date: &T) -> String {
    format!("{:0>4}-{:0>2}-{:0>2}", date.year(), date.month(), date.day())
}

impl FromSql<Date, Sqlite> for NaiveDate {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        let text = not_none!(value).read_text();
        parse_sqlite_date(text)
    }
}

impl ToSql<Date, Sqlite> for NaiveDate {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> Result<IsNull, Box<Error+Send+Sync>> {
        let s = dump_sqlite_date(self);
        ToSql::<Text, Sqlite>::to_sql(&s, out)
    }
}

impl FromSql<Time, Sqlite> for NaiveTime {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        let text = not_none!(value).read_text();
        let time = NaiveTime::parse_from_str(text, SQLITE_TIME_FMT)?;
        Ok(time)
    }
}

impl ToSql<Time, Sqlite> for NaiveTime {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> Result<IsNull, Box<Error+Send+Sync>> {
        let s = format!("{}", self.format(SQLITE_TIME_FMT));
        ToSql::<Text, Sqlite>::to_sql(&s, out)
    }
}

impl FromSql<Timestamp, Sqlite> for NaiveDateTime {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        let text = not_none!(value).read_text();
        let dt: Vec<&str> = text.split(' ').collect();

        if dt.len() != 2 {
            return Err("Can't parse Datetime. Too many parts. It should be in the form: YYYY-MM-DD HH:MM:SS".into())
        }

        let time = NaiveTime::parse_from_str(dt[1], SQLITE_TIME_FMT)?;
        let datetime = parse_sqlite_date(dt[0])?.and_time(time);

        Ok(datetime)
    }
}

impl ToSql<Timestamp, Sqlite> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> Result<IsNull, Box<Error+Send+Sync>> {
        ToSql::<Date, Sqlite>::to_sql(&self.date(), out)?;
        write!(out, " ")?;
        ToSql::<Time, Sqlite>::to_sql(&self.time(), out)
    }
}


#[cfg(test)]
mod tests {
    extern crate dotenv;
    extern crate chrono;

    use self::chrono::{Duration, NaiveDate, NaiveTime, NaiveDateTime, Utc, Timelike};
    use self::chrono::naive::MAX_DATE;
    use self::dotenv::dotenv;

    use ::select;
    use expression::dsl::{sql, now};
    use prelude::*;
    use types::{Date, Time, Timestamp};

    fn connection() -> SqliteConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("SQLITE_DATABASE_URL")
            .or_else(|_| ::std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        SqliteConnection::establish(&connection_url).unwrap()
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let query = select(sql::<Timestamp>("'1970-01-01 00:00:00.000000'").eq(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }


    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql = select(sql::<Timestamp>("'1970-1-1 00:00:00'"))
            .get_result(&connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = Utc::now().naive_utc() + Duration::seconds(60);
        let query = select(now.lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = Utc::now().naive_utc() - Duration::seconds(600);
        let query = select(now.gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_encode_correctly() {
        let connection = connection();

        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(sql::<Time>("'00:00:00.000000'").eq(midnight));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("'12:00:00.000000'").eq(noon));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn times_of_day_decode_correctly() {
        let connection = connection();
        let midnight = NaiveTime::from_hms(0, 0, 0);
        let query = select(sql::<Time>("'00:00:00'"));
        assert_eq!(Ok(midnight), query.get_result::<NaiveTime>(&connection));

        let noon = NaiveTime::from_hms(12, 0, 0);
        let query = select(sql::<Time>("'12:00:00'"));
        assert_eq!(Ok(noon), query.get_result::<NaiveTime>(&connection));

        let roughly_half_past_eleven = NaiveTime::from_hms_micro(23, 37, 4, 2200);
        let query = select(sql::<Time>("'23:37:04.002200'"));
        assert_eq!(Ok(roughly_half_past_eleven), query.get_result::<NaiveTime>(&connection));
    }

    #[test]
    fn dates_encode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("'2000-01-01'").eq(january_first_2000));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_past = NaiveDate::from_ymd(-398, 4, 11);
        let query = select(sql::<Date>("'-398-04-11'").eq(distant_past));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let max_date = MAX_DATE;
        let query = select(sql::<Date>("'262143-12-31'").eq(max_date));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-01-01'").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_future = NaiveDate::from_ymd(72_400, 1, 8);
        let query = select(sql::<Date>("'72400-01-08'").eq(distant_future));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn dates_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'"));
        assert_eq!(Ok(january_first_2000), query.get_result::<NaiveDate>(&connection));

        let distant_past = NaiveDate::from_ymd(-399, 4, 11);
        let query = select(sql::<Date>("'-399-04-11'"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDate>(&connection));

        let max_date = MAX_DATE;
        let query = select(sql::<Date>("'262143-12-31'"));
        assert_eq!(Ok(max_date), query.get_result::<NaiveDate>(&connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-01-01'"));
        assert_eq!(Ok(january_first_2018), query.get_result::<NaiveDate>(&connection));

        let distant_future = NaiveDate::from_ymd(72_400, 1, 8);
        let query = select(sql::<Date>("'72400-01-08'"));
        assert_eq!(Ok(distant_future), query.get_result::<NaiveDate>(&connection));
    }

    #[test]
    fn datetimes_decode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1).and_hms(1, 1, 1);
        let query = select(sql::<Timestamp>("'2000-1-1 01:01:01.000000'"));
        assert_eq!(Ok(january_first_2000), query.get_result::<NaiveDateTime>(&connection));

        let distant_past = NaiveDate::from_ymd(-399, 4, 11).and_hms(2, 2, 2);
        let query = select(sql::<Timestamp>("'-399-04-11 02:02:02.000000'"));
        assert_eq!(Ok(distant_past), query.get_result::<NaiveDateTime>(&connection));

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1);
        let query = select(sql::<Date>("'2018-01-01'"));
        assert_eq!(Ok(january_first_2018), query.get_result::<NaiveDate>(&connection));

        let distant_future = NaiveDate::from_ymd(72_400, 1, 8).and_hms(23, 59, 59).with_nanosecond(100_000).unwrap();
        let query = select(sql::<Timestamp>("'72400-01-08 23:59:59.000100'"));
        assert_eq!(Ok(distant_future), query.get_result::<NaiveDateTime>(&connection));
   }

    #[test]
    fn datetimes_encode_correctly() {
        let connection = connection();
        let january_first_2000 = NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);
        let query = select(sql::<Timestamp>("'2000-01-01 00:00:00.000000'").eq(january_first_2000));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_past = NaiveDate::from_ymd(-398, 4, 11).and_hms(20, 00, 20);
        let query = select(sql::<Timestamp>("'-398-04-11 20:00:20.000000'").eq(distant_past));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let january_first_2018 = NaiveDate::from_ymd(2018, 1, 1).and_hms(12, 00, 00).with_nanosecond(500_000).unwrap();
        let query = select(sql::<Timestamp>("'2018-01-01 12:00:00.000500'").eq(january_first_2018));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let distant_future = NaiveDate::from_ymd(72_400, 1, 8).and_hms(0, 0, 0);
        let query = select(sql::<Timestamp>("'72400-01-08 00:00:00.000000'").eq(distant_future));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }
}
