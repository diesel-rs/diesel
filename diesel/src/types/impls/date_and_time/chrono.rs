//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.
extern crate chrono;

use std::error::Error;
use std::io::Write;
use self::chrono::{Duration, NaiveDateTime, NaiveDate};

use expression::AsExpression;
use expression::bound::Bound;
use query_source::Queriable;
use super::PgTimestamp;
use types::{self, FromSql, IsNull, Timestamp, ToSql};

expression_impls! {
    Timestamp -> NaiveDateTime,
}

queriable_impls! {
    Timestamp -> NaiveDateTime,
}

// Postgres timestamps start from January 1st 2000.
fn pg_epoch() -> NaiveDateTime {
    NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0)
}

impl FromSql<Timestamp> for NaiveDateTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let PgTimestamp(offset) = try!(FromSql::<Timestamp>::from_sql(bytes));
        Ok(pg_epoch() + Duration::microseconds(offset))
    }
}

impl ToSql<Timestamp> for NaiveDateTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let time = match (*self - pg_epoch()).num_microseconds() {
            Some(time) => time,
            None => {
                let error_message = format!("{:?} as microseconds is too large to fit in an i64", self);
                return Err(Box::<Error + Send + Sync>::from(error_message));
            }
        };
        ToSql::<Timestamp>::to_sql(&PgTimestamp(time), out)
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;
    extern crate chrono;

    use connection::Connection;
    use self::dotenv::dotenv;
    use self::chrono::*;
    use types::{Bool, Timestamp};

    fn connection() -> Connection {
        dotenv().ok();

        let connection_url = ::std::env::var("DATABASE_URL").ok()
            .expect("DATABASE_URL must be set in order to run tests");
        Connection::establish(&connection_url).unwrap()
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        assert!(
            connection.query_sql_params::<Bool, bool, Timestamp, NaiveDateTime>(
                "SELECT $1::timestamp = '1970-01-01'", &time)
            .unwrap().nth(0).unwrap()
        );
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql = connection.query_sql::<Timestamp, NaiveDateTime>("SELECT '1970-01-01'::timestamp")
            .unwrap().nth(0).unwrap();
        assert_eq!(time, epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = UTC::now().naive_utc() + Duration::seconds(60);
        assert!(
            connection.query_sql_params::<Bool, bool, Timestamp, NaiveDateTime>(
                "SELECT $1::timestamp > current_timestamp at time zone 'utc'", &time)
            .unwrap().nth(0).unwrap()
        );

        let time = UTC::now().naive_utc() - Duration::seconds(60);
        assert!(
            connection.query_sql_params::<Bool, bool, Timestamp, NaiveDateTime>(
                "SELECT $1::timestamp < current_timestamp at time zone 'utc'", &time)
            .unwrap().nth(0).unwrap()
        );
    }
}
