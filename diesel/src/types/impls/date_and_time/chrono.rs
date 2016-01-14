//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.

extern crate chrono;
extern crate byteorder as localbyteorder; // conflicts otherwise

use std::error::Error;
use std::io::Write;
use self::localbyteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use self::chrono::{Duration, NaiveDate, NaiveDateTime, DateTime, UTC, Local, FixedOffset};

use expression::{Expression, NonAggregate};
use query_builder::{QueryBuilder, BuildQueryResult};
use types::{FromSql, IsNull, Timestamp, ToSql};
use types::impls::option::UnexpectedNullError;


// Postgres timestamps start from January 1st 2000.
fn base() -> NaiveDateTime {
    NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0)
}

impl Expression for NaiveDateTime {
    type SqlType = Timestamp;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        let formatted_string = format!("{}", self.format("%Y-%m-%d %H:%M:%S"));
        out.push_sql(&formatted_string);
        Ok(())
    }
}

impl NonAggregate for NaiveDateTime {}

impl FromSql<Timestamp> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        let offset: i64 = try!(bytes.read_i64::<BigEndian>());
        Ok(base() + Duration::microseconds(offset))
    }
}

impl FromSql<Timestamp> for DateTime<UTC> {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let naive = try!(NaiveDateTime::from_sql(bytes));
        Ok(DateTime::from_utc(naive, UTC))
    }
}

impl FromSql<Timestamp> for DateTime<Local> {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let utc = try!(DateTime::<UTC>::from_sql(bytes));
        Ok(utc.with_timezone(&Local))
    }
}

impl FromSql<Timestamp> for DateTime<FixedOffset> {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let utc = try!(DateTime::<UTC>::from_sql(bytes));
        Ok(utc.with_timezone(&FixedOffset::east(0))) // i.e. UTC
    }
}

impl ToSql<Timestamp> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let time = match (*self - base()).num_microseconds() {
            Some(time) => time,
            None => {
                let err: Box<Error + Send + Sync> = "value is too large to transmit".into();
                return Err(err);
            }
        };
        try!(out.write_i64::<BigEndian>(time));
        Ok(IsNull::No)
    }
}

impl ToSql<Timestamp> for DateTime<UTC> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let naive: NaiveDateTime = self.naive_utc();
        ToSql::<Timestamp>::to_sql(&naive, out)
    }
}

impl ToSql<Timestamp> for DateTime<FixedOffset> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let naive: NaiveDateTime = self.naive_utc();
        ToSql::<Timestamp>::to_sql(&naive, out)
    }
}

impl ToSql<Timestamp> for DateTime<Local> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let naive: NaiveDateTime = self.naive_utc();
        ToSql::<Timestamp>::to_sql(&naive, out)
    }
}

#[test]
fn utc_datetime_to_sql() {
    use self::chrono::TimeZone;

    let mut bytes = vec![];
    ToSql::<Timestamp>::to_sql(&UTC.ymd(2000, 1, 1).and_hms(0, 0, 0), &mut bytes).unwrap();
    ToSql::<Timestamp>::to_sql(&UTC.ymd(2010, 12, 4).and_hms(14, 39, 6), &mut bytes).unwrap();
    ToSql::<Timestamp>::to_sql(&UTC.ymd(2032, 2, 4).and_hms(12, 59, 59), &mut bytes).unwrap();
    ToSql::<Timestamp>::to_sql(&UTC.ymd(1789, 7, 14).and_hms(17, 30, 22), &mut bytes).unwrap();
    assert_eq!(bytes,
               vec![
               0,0,0,0,0,0,0,0,
               0x00,0x01,0x39,0x95,0x62,0xba,0x56,0x80,
               0x00,0x03,0x99,0x29,0x4d,0x41,0xd1,0xc0,
               0xff,0xe8,0x67,0x82,0x01,0x2b,0xc7,0x80,
               ]);
}
