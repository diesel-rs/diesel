extern crate quickcheck;

use self::quickcheck::{Arbitrary, Gen};

use super::{PgDate, PgInterval, PgTime, PgTimestamp};

// see https://www.postgresql.org/docs/current/datatype-datetime.html
// for the specific limits

impl Arbitrary for PgDate {
    fn arbitrary(g: &mut Gen) -> Self {
        const MIN_DAY: i32 = (-4713 * 365) - (2000 * 365);
        const MAX_DAY: i32 = 5874897 * 365 - (2000 * 365);

        let mut day = i32::arbitrary(g);

        if day <= MIN_DAY {
            day %= MIN_DAY;
        }

        if day >= MAX_DAY {
            day %= MAX_DAY;
        }

        PgDate(day)
    }
}

impl Arbitrary for PgTime {
    fn arbitrary(g: &mut Gen) -> Self {
        // 24:00:00 in microseconds
        const MAX_TIME: i64 = 24 * 60 * 60 * 1_000_000;

        let time = u64::arbitrary(g);

        let mut time = if time > i64::MAX as u64 {
            (time / 2) as i64
        } else {
            time as i64
        };

        if time > MAX_TIME {
            time %= MAX_TIME;
        }

        PgTime(time)
    }
}

impl Arbitrary for PgTimestamp {
    fn arbitrary(g: &mut Gen) -> Self {
        const MIN_TIMESTAMP: i64 = -4713 * 365 * 24 * 60 * 60 * 100_000;
        const MAX_TIMESTAMP: i64 = 294276 * 365 * 24 * 60 * 60 * 100_000;

        let mut timestamp = i64::arbitrary(g);

        if timestamp <= MIN_TIMESTAMP {
            timestamp %= MIN_TIMESTAMP;
        }

        if timestamp >= MAX_TIMESTAMP {
            timestamp %= MAX_TIMESTAMP;
        }

        PgTimestamp(timestamp)
    }
}

impl Arbitrary for PgInterval {
    fn arbitrary(g: &mut Gen) -> Self {
        PgInterval {
            microseconds: i64::arbitrary(g),
            days: i32::arbitrary(g),
            months: i32::arbitrary(g),
        }
    }
}
