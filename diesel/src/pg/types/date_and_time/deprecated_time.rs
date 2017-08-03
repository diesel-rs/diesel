extern crate time;

use std::error::Error;
use std::io::Write;

use self::time::{Timespec, Duration};

use pg::Pg;
use types::{self, ToSql, ToSqlOutput, FromSql, IsNull, Timestamp};

expression_impls!(Timestamp -> Timespec);
queryable_impls!(Timestamp -> Timespec);

const TIME_SEC_CONV: i64 = 946684800;

impl ToSql<types::Timestamp, Pg> for Timespec {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        let pg_epoch = Timespec::new(TIME_SEC_CONV, 0);
        let duration = *self - pg_epoch;
        let t = try!(duration.num_microseconds().ok_or("Overflow error"));
        ToSql::<types::BigInt, Pg>::to_sql(&t, out)
    }
}

impl FromSql<types::Timestamp, Pg> for Timespec {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let t = try!(<i64 as FromSql<types::BigInt, Pg>>::from_sql(bytes));
        let pg_epoch = Timespec::new(TIME_SEC_CONV, 0);
        let duration = Duration::microseconds(t);
        let out = pg_epoch + duration;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;
    extern crate time;

    use self::dotenv::dotenv;
    use self::time::{Timespec, Duration};

    use ::select;
    use expression::dsl::{sql, now};
    use prelude::*;
    use types::Timestamp;

    fn connection() -> PgConnection {
        dotenv().ok();

        let connection_url = ::std::env::var("PG_DATABASE_URL")
            .or_else(|_| ::std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        PgConnection::establish(&connection_url).unwrap()
    }

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = connection();
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(Timespec::new(0, 0)));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let epoch_from_sql = select(sql::<Timestamp>("'1970-01-01'::timestamp"))
            .get_result::<Timespec>(&connection);
        assert_eq!(Ok(Timespec::new(0, 0)), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = time::now_utc().to_timespec() + Duration::seconds(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = time::now_utc().to_timespec() - Duration::seconds(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }
}
