use std::error::Error;
use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pg::Pg;
use types::{self, FromSql, IsNull, Timestamp, ToSql, ToSqlOutput};

expression_impls!(Timestamp -> SystemTime);

fn pg_epoch() -> SystemTime {
    let thirty_years = Duration::from_secs(946_684_800);
    UNIX_EPOCH + thirty_years
}

impl ToSql<types::Timestamp, Pg> for SystemTime {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        let (before_epoch, duration) = match self.duration_since(pg_epoch()) {
            Ok(duration) => (false, duration),
            Err(time_err) => (true, time_err.duration()),
        };
        let time_since_epoch = if before_epoch {
            -(duration_to_usecs(duration) as i64)
        } else {
            duration_to_usecs(duration) as i64
        };
        ToSql::<types::BigInt, Pg>::to_sql(&time_since_epoch, out)
    }
}

impl FromSql<types::Timestamp, Pg> for SystemTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        let usecs_passed = try!(<i64 as FromSql<types::BigInt, Pg>>::from_sql(bytes));
        let before_epoch = usecs_passed < 0;
        let time_passed = usecs_to_duration(usecs_passed.abs() as u64);

        if before_epoch {
            Ok(pg_epoch() - time_passed)
        } else {
            Ok(pg_epoch() + time_passed)
        }
    }
}

const USEC_PER_SEC: u64 = 1_000_000;
const NANO_PER_USEC: u32 = 1_000;

fn usecs_to_duration(usecs_passed: u64) -> Duration {
    let usecs_passed = usecs_passed;
    let seconds = usecs_passed / USEC_PER_SEC;
    let subsecond_usecs = usecs_passed % USEC_PER_SEC;
    let subseconds = subsecond_usecs as u32 * NANO_PER_USEC;
    Duration::new(seconds, subseconds)
}

fn duration_to_usecs(duration: Duration) -> u64 {
    let seconds = duration.as_secs() * USEC_PER_SEC;
    let subseconds = duration.subsec_nanos() / NANO_PER_USEC;
    seconds + u64::from(subseconds)
}

#[cfg(test)]
mod tests {
    extern crate dotenv;

    use self::dotenv::dotenv;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use select;
    use dsl::{now, sql};
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
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(UNIX_EPOCH));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = connection();
        let epoch_from_sql = select(sql::<Timestamp>("'1970-01-01'::timestamp"))
            .get_result::<SystemTime>(&connection);
        assert_eq!(Ok(UNIX_EPOCH), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = connection();
        let time = SystemTime::now() + Duration::from_secs(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());

        let time = SystemTime::now() - Duration::from_secs(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(&connection).unwrap());
    }
}
