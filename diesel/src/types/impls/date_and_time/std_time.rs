use std::error::Error;
use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use expression::bound::Bound;
use expression::AsExpression;
use types::{self, ToSql, FromSql, IsNull, NativeSqlType};
use query_source::Queriable;

expression_impls! {
    Timestamp -> SystemTime,
}

queriable_impls! {
    Timestamp -> SystemTime,
}

fn pg_epoch() -> SystemTime {
    let thirty_years = Duration::from_secs(946684800);
    UNIX_EPOCH + thirty_years
}

impl ToSql<types::Timestamp> for SystemTime {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let (before_epoch, duration) = match self.duration_from_earlier(pg_epoch()) {
            Ok(duration) => (false, duration),
            Err(time_err) => (true, time_err.duration()),
        };
        let time_since_epoch = if before_epoch {
            -(duration_to_usecs(duration) as i64)
        } else {
            duration_to_usecs(duration) as i64
        };
        ToSql::<types::BigInt>::to_sql(&time_since_epoch, out)
    }
}

impl FromSql<types::Timestamp> for SystemTime {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let usecs_passed = try!(<i64 as FromSql<types::BigInt>>::from_sql(bytes));
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
    seconds + subseconds as u64
}
