use sqlite::{Sqlite, SqliteType};
use types;

impl types::HasSqlType<types::Date> for Sqlite {
    fn metadata() -> SqliteType {
        SqliteType::Text
    }
}

impl types::HasSqlType<types::Time> for Sqlite {
    fn metadata() -> SqliteType {
        SqliteType::Text
    }
}

impl types::HasSqlType<types::Timestamp> for Sqlite {
    fn metadata() -> SqliteType {
        SqliteType::Text
    }
}

#[cfg(feature = "chrono")]
mod chrono {
    extern crate chrono;

    use std::error::Error;
    use std::io::Write;
    use self::chrono::{NaiveDateTime, NaiveDate, NaiveTime};

    use sqlite::Sqlite;
    use sqlite::connection::SqliteValue;
    use types::{self, Date, FromSql, IsNull, Time, Timestamp, ToSql, Text};

    const SQLITE_DATETIME_FMT: &'static str = "%Y-%m-%d %H:%M:%S";
    const SQLITE_DATE_FMT: &'static str = "%Y-%m-%d";
    const SQLITE_TIME_FMT: &'static str = "%H:%M:%S";

    expression_impls! {
        Date -> NaiveDate,
        Time -> NaiveTime,
        Timestamp -> NaiveDateTime,
    }

    queryable_impls! {
        Date -> NaiveDate,
        Time -> NaiveTime,
        Timestamp -> NaiveDateTime,
    }

    impl FromSql<Date, Sqlite> for NaiveDate {
        fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
            let text = not_none!(value).read_text();
            let date = try!(NaiveDate::parse_from_str(text, SQLITE_DATE_FMT));
            Ok(date)
        }
    }

    impl ToSql<Timestamp, Sqlite> for NaiveDate {
        fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
            let s = format!("{}", self.format(SQLITE_DATE_FMT));
            ToSql::<Text, Sqlite>::to_sql(&s, out)
        }
    }

    impl FromSql<Time, Sqlite> for NaiveTime {
        fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
            let text = not_none!(value).read_text();
            let time = try!(NaiveTime::parse_from_str(text, SQLITE_TIME_FMT));
            Ok(time)
        }
    }

    impl ToSql<Time, Sqlite> for NaiveTime {
        fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
            let s = format!("{}", self.format(SQLITE_TIME_FMT));
            ToSql::<Text, Sqlite>::to_sql(&s, out)
        }
    }

    impl FromSql<Timestamp, Sqlite> for NaiveDateTime {
        fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
            let text = not_none!(value).read_text();
            let datetime = try!(NaiveDateTime::parse_from_str(text, SQLITE_DATETIME_FMT));
            Ok(datetime)
        }
    }

    impl ToSql<Timestamp, Sqlite> for NaiveDateTime {
        fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
            let s = format!("{}", self.format(SQLITE_DATETIME_FMT));
            ToSql::<Text, Sqlite>::to_sql(&s, out)
        }
    }


}