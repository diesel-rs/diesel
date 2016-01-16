//! This module makes it possible to map `chrono::DateTime` values to postgres `Date`
//! and `Timestamp` fields. It is enabled with the `chrono` feature.
extern crate chrono;

use std::error::Error;
use std::io::Write;
use self::chrono::{Duration, NaiveDateTime, NaiveDate};

use expression::AsExpression;
use expression::bound::Bound;
use query_source::Queryable;
use super::PgTimestamp;
use types::{self, FromSql, IsNull, Timestamp, ToSql};

expression_impls! {
    Timestamp -> NaiveDateTime,
}

queryable_impls! {
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

    use self::chrono::*;
    use self::dotenv::dotenv;

    use ::select;
    use connection::Connection;
    use expression::dsl::{sql, now};
    use prelude::*;
    use types::Timestamp;

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
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let time = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        let epoch_from_sql = select(sql::<Timestamp>("'1970-01-01'::timestamp"))
            .get_result(&connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = UTC::now().naive_utc() + Duration::seconds(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = UTC::now().naive_utc() - Duration::seconds(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }
}
