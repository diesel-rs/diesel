use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types;

fn pg_epoch() -> SystemTime {
    let thirty_years = Duration::from_secs(946_684_800);
    UNIX_EPOCH + thirty_years
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Timestamp, Pg> for SystemTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let (before_epoch, duration) = match self.duration_since(pg_epoch()) {
            Ok(duration) => (false, duration),
            Err(time_err) => (true, time_err.duration()),
        };
        let time_since_epoch = if before_epoch {
            -(duration_to_usecs(duration) as i64)
        } else {
            duration_to_usecs(duration) as i64
        };
        ToSql::<sql_types::BigInt, Pg>::to_sql(&time_since_epoch, &mut out.reborrow())
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Timestamp, Pg> for SystemTime {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let usecs_passed = <i64 as FromSql<sql_types::BigInt, Pg>>::from_sql(bytes)?;
        let before_epoch = usecs_passed < 0;
        let time_passed = usecs_to_duration(usecs_passed.unsigned_abs());

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
    let seconds = usecs_passed / USEC_PER_SEC;
    let subsecond_usecs = usecs_passed % USEC_PER_SEC;
    let subseconds = subsecond_usecs as u32 * NANO_PER_USEC;
    Duration::new(seconds, subseconds)
}

fn duration_to_usecs(duration: Duration) -> u64 {
    let seconds = duration.as_secs() * USEC_PER_SEC;
    let subseconds = duration.subsec_micros();
    seconds + u64::from(subseconds)
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::Timestamp;
    use crate::test_helpers::pg_connection;

    #[test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut pg_connection();
        let query = select(sql::<Timestamp>("'1970-01-01'").eq(UNIX_EPOCH));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut pg_connection();
        let epoch_from_sql = select(sql::<Timestamp>("'1970-01-01'::timestamp"))
            .get_result::<SystemTime>(connection);
        assert_eq!(Ok(UNIX_EPOCH), epoch_from_sql);
    }

    #[test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut pg_connection();
        let time = SystemTime::now() + Duration::from_secs(60);
        let query = select(now.at_time_zone("utc").lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = SystemTime::now() - Duration::from_secs(60);
        let query = select(now.at_time_zone("utc").gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }
}
