extern crate quickcheck;

use self::quickcheck::{Arbitrary, Gen};

use super::{PgDate, PgInterval, PgTime, PgTimestamp};

impl Arbitrary for PgDate {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        PgDate(i32::arbitrary(g))
    }
}

impl Arbitrary for PgTime {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut time = -1;
        while time < 0 {
            time = i64::arbitrary(g);
        }
        PgTime(time)
    }
}

impl Arbitrary for PgTimestamp {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        PgTimestamp(i64::arbitrary(g))
    }
}

impl Arbitrary for PgInterval {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        PgInterval {
            microseconds: i64::arbitrary(g),
            days: i32::arbitrary(g),
            months: i32::arbitrary(g),
        }
    }
}
