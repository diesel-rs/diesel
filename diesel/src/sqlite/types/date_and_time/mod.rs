use crate::deserialize::{self, FromSql};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types;
use crate::sqlite::connection::SqliteValue;
use crate::sqlite::Sqlite;

#[cfg(feature = "chrono")]
mod chrono;
#[cfg(feature = "time")]
mod time;

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Date, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Date, Sqlite> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Date, Sqlite> for String {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <str as ToSql<sql_types::Date, Sqlite>>::to_sql(self as &str, out)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Time, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Time, Sqlite> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Time, Sqlite> for String {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <str as ToSql<sql_types::Time, Sqlite>>::to_sql(self as &str, out)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Timestamp, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Timestamp, Sqlite> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Timestamp, Sqlite> for String {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <str as ToSql<sql_types::Timestamp, Sqlite>>::to_sql(self as &str, out)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::TimestamptzSqlite, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::TimestamptzSqlite, Sqlite> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::TimestamptzSqlite, Sqlite> for String {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <str as ToSql<sql_types::TimestamptzSqlite, Sqlite>>::to_sql(self as &str, out)
    }
}

#[cfg(all(test, feature = "chrono", feature = "time"))]
mod tests {
    extern crate chrono;
    extern crate time;

    use chrono::{
        DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc,
    };
    use time::{
        macros::{date, datetime, offset, time},
        Date, OffsetDateTime, PrimitiveDateTime, Time,
    };

    use crate::insert_into;
    use crate::prelude::*;
    use crate::test_helpers::connection;

    crate::table! {
        table_timestamp_tz(id) {
            id -> Integer,
            timestamp_with_tz -> TimestamptzSqlite,
        }
    }
    crate::table! {
      table_timestamp(id) {
          id -> Integer,
          timestamp -> Timestamp
      }
    }
    crate::table! {
        table_date(id) {
            id -> Integer,
            date -> Date
        }
    }
    crate::table! {
        table_time(id) {
            id -> Integer,
            time -> Time
        }
    }

    fn eq_date(left: Date, right: NaiveDate) -> bool {
        left.year() == right.year()
            && left.month() as u8 == right.month() as u8
            && left.day() == right.day() as u8
    }

    fn eq_time(left: Time, right: NaiveTime) -> bool {
        left.hour() == right.hour() as u8
            && left.minute() == right.minute() as u8
            && left.second() == right.second() as u8
            && left.nanosecond() == right.nanosecond()
    }

    fn eq_datetime(left: PrimitiveDateTime, right: NaiveDateTime) -> bool {
        eq_date(left.date(), right.date()) && eq_time(left.time(), right.time())
    }

    fn eq_datetime_utc(left: OffsetDateTime, right: DateTime<Utc>) -> bool {
        left.unix_timestamp_nanos() == right.timestamp_nanos_opt().unwrap() as i128
    }

    fn eq_datetime_offset(left: OffsetDateTime, right: DateTime<FixedOffset>) -> bool {
        left.unix_timestamp_nanos() == right.timestamp_nanos_opt().unwrap() as i128
    }

    fn create_tables(conn: &mut SqliteConnection) {
        crate::sql_query(
            "CREATE TABLE table_timestamp_tz(id INTEGER PRIMARY KEY, timestamp_with_tz TEXT);",
        )
        .execute(conn)
        .unwrap();

        crate::sql_query("CREATE TABLE table_timestamp(id INTEGER PRIMARY KEY, timestamp TEXT);")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE table_date(id INTEGER PRIMARY KEY, date TEXT);")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE table_time(id INTEGER PRIMARY KEY, time TEXT);")
            .execute(conn)
            .unwrap();
    }

    #[test]
    fn time_to_chrono_date() {
        let conn = &mut connection();
        create_tables(conn);

        let original = date!(2000 - 1 - 1);

        insert_into(table_date::table)
            .values(vec![(table_date::id.eq(1), table_date::date.eq(original))])
            .execute(conn)
            .unwrap();

        let translated = table_date::table
            .select(table_date::date)
            .get_result::<NaiveDate>(conn)
            .unwrap();

        assert!(eq_date(original, translated))
    }

    #[test]
    fn chrono_to_time_date() {
        let conn = &mut connection();
        create_tables(conn);

        let original = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();

        insert_into(table_date::table)
            .values(vec![(table_date::id.eq(1), table_date::date.eq(original))])
            .execute(conn)
            .unwrap();

        let translated = table_date::table
            .select(table_date::date)
            .get_result::<Date>(conn)
            .unwrap();

        assert!(eq_date(translated, original))
    }

    #[test]
    fn time_to_chrono_time() {
        let conn = &mut connection();
        create_tables(conn);

        let original = time!(1:1:1.001);

        insert_into(table_time::table)
            .values(vec![(table_time::id.eq(1), table_time::time.eq(original))])
            .execute(conn)
            .unwrap();

        let translated = table_time::table
            .select(table_time::time)
            .get_result::<NaiveTime>(conn)
            .unwrap();

        assert!(eq_time(original, translated))
    }

    #[test]
    fn chrono_to_time_time() {
        let conn = &mut connection();
        create_tables(conn);

        let original = NaiveTime::from_hms_milli_opt(1, 1, 1, 1).unwrap();

        insert_into(table_time::table)
            .values(vec![(table_time::id.eq(1), table_time::time.eq(original))])
            .execute(conn)
            .unwrap();

        let translated = table_time::table
            .select(table_time::time)
            .get_result::<Time>(conn)
            .unwrap();

        assert!(eq_time(translated, original))
    }

    #[test]
    fn time_to_chrono_datetime() {
        let conn = &mut connection();
        create_tables(conn);

        let original = datetime!(2000-1-1 1:1:1.001);

        insert_into(table_timestamp::table)
            .values(vec![(
                table_timestamp::id.eq(1),
                table_timestamp::timestamp.eq(original),
            )])
            .execute(conn)
            .unwrap();

        let translated = table_timestamp::table
            .select(table_timestamp::timestamp)
            .get_result::<NaiveDateTime>(conn)
            .unwrap();

        assert!(eq_datetime(original, translated))
    }

    #[test]
    fn chrono_to_time_datetime() {
        let conn = &mut connection();
        create_tables(conn);

        let original = NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_milli_opt(1, 1, 1, 1)
            .unwrap();

        insert_into(table_timestamp::table)
            .values(vec![(
                table_timestamp::id.eq(1),
                table_timestamp::timestamp.eq(original),
            )])
            .execute(conn)
            .unwrap();

        let translated = table_timestamp::table
            .select(table_timestamp::timestamp)
            .get_result::<PrimitiveDateTime>(conn)
            .unwrap();

        assert!(eq_datetime(translated, original))
    }

    #[test]
    fn chrono_to_time_datetime_utc() {
        let conn = &mut connection();
        create_tables(conn);

        let original = Utc::now();

        insert_into(table_timestamp_tz::table)
            .values(vec![(
                table_timestamp_tz::id.eq(1),
                table_timestamp_tz::timestamp_with_tz.eq(original),
            )])
            .execute(conn)
            .unwrap();

        let translated = table_timestamp_tz::table
            .select(table_timestamp_tz::timestamp_with_tz)
            .get_result::<OffsetDateTime>(conn)
            .unwrap();

        assert!(eq_datetime_utc(translated, original))
    }

    #[test]
    fn time_to_chrono_datetime_utc() {
        let conn = &mut connection();
        create_tables(conn);

        let original = OffsetDateTime::now_utc();

        insert_into(table_timestamp_tz::table)
            .values(vec![(
                table_timestamp_tz::id.eq(1),
                table_timestamp_tz::timestamp_with_tz.eq(original),
            )])
            .execute(conn)
            .unwrap();

        let translated = table_timestamp_tz::table
            .select(table_timestamp_tz::timestamp_with_tz)
            .get_result::<DateTime<Utc>>(conn)
            .unwrap();

        assert!(eq_datetime_utc(original, translated))
    }

    #[test]
    fn chrono_to_time_datetime_timezone() {
        let conn = &mut connection();
        create_tables(conn);

        let original = Utc::now().with_timezone(&FixedOffset::east_opt(5 * 3600).unwrap());

        insert_into(table_timestamp_tz::table)
            .values(vec![(
                table_timestamp_tz::id.eq(1),
                table_timestamp_tz::timestamp_with_tz.eq(original),
            )])
            .execute(conn)
            .unwrap();

        let translated = table_timestamp_tz::table
            .select(table_timestamp_tz::timestamp_with_tz)
            .get_result::<OffsetDateTime>(conn)
            .unwrap();

        assert!(eq_datetime_offset(translated, original))
    }

    #[test]
    fn time_to_chrono_datetime_offset() {
        let conn = &mut connection();
        create_tables(conn);

        let original = OffsetDateTime::now_utc().to_offset(offset!(+5));

        insert_into(table_timestamp_tz::table)
            .values(vec![(
                table_timestamp_tz::id.eq(1),
                table_timestamp_tz::timestamp_with_tz.eq(original),
            )])
            .execute(conn)
            .unwrap();

        let translated = table_timestamp_tz::table
            .select(table_timestamp_tz::timestamp_with_tz)
            .get_result::<DateTime<Utc>>(conn)
            .unwrap();

        assert!(eq_datetime_utc(original, translated))
    }
}
