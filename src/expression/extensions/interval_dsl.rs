use std::ops::Mul;

use types::structs::PgInterval;

pub trait MicroIntervalDsl: Sized + Mul<i64, Output=Self> {
    fn microseconds(self) -> PgInterval;

    fn milliseconds(self) -> PgInterval {
        (self * 1000).microseconds()
    }

    fn seconds(self) -> PgInterval {
        (self * 1000).milliseconds()
    }

    fn minutes(self) -> PgInterval {
        (self * 60).seconds()
    }

    fn hours(self) -> PgInterval {
        (self * 60).minutes()
    }

    fn microsecond(self) -> PgInterval {
        self.microseconds()
    }

    fn millisecond(self) -> PgInterval {
        self.milliseconds()
    }

    fn second(self) -> PgInterval {
        self.seconds()
    }

    fn minute(self) -> PgInterval {
        self.minutes()
    }

    fn hour(self) -> PgInterval {
        self.hours()
    }
}

pub trait DayAndMonthIntervalDsl: Sized + Mul<i32, Output=Self> {
    fn days(self) -> PgInterval;
    fn months(self) -> PgInterval;

    fn weeks(self) -> PgInterval {
        (self * 7).days()
    }

    fn years(self) -> PgInterval {
        (self * 12).months()
    }

    fn day(self) -> PgInterval {
        self.days()
    }

    fn week(self) -> PgInterval {
        self.weeks()
    }

    fn month(self) -> PgInterval {
        self.months()
    }

    fn year(self) -> PgInterval {
        self.years()
    }
}

impl MicroIntervalDsl for i64 {
    fn microseconds(self) -> PgInterval {
        PgInterval {
            microseconds: self,
            days: 0,
            months: 0,
        }
    }
}

impl DayAndMonthIntervalDsl for i32 {
    fn days(self) -> PgInterval {
        PgInterval {
            microseconds: 0,
            days: self,
            months: 0,
        }
    }

    fn months(self) -> PgInterval {
        PgInterval {
            microseconds: 0,
            days: 0,
            months: self,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate quickcheck;
    use self::quickcheck::quickcheck;
    use super::*;
    use connection::Connection;
    use types;

    macro_rules! test_fn {
        ($tpe:ty, $test_name:ident, $units:ident) => {
            fn $test_name(val: $tpe) -> bool {
                let connection_url = ::std::env::var("DATABASE_URL").ok()
                    .expect("DATABASE_URL must be set in order to run tests");
                let connection = Connection::establish(&connection_url).unwrap();

                let query = format!(concat!("SELECT '{}", stringify!($units), "'::interval"), val);
                let res = connection.query_sql::<types::Interval, _>(&query).unwrap().nth(0).unwrap();
                val.$units() == res
            }

            quickcheck($test_name as fn($tpe) -> bool);
        }
    }

    #[test]
    fn micro_intervals_match_pg_values_i64() {
        test_fn!(i64, test_microseconds, microseconds);
        test_fn!(i64, test_milliseconds, milliseconds);
        test_fn!(i64, test_seconds, seconds);
        test_fn!(i64, test_minutes, minutes);
        test_fn!(i64, test_hours, hours);
    }

    #[test]
    fn day_and_month_intervals_match_pg_values_i32() {
        test_fn!(i32, test_days, days);
        test_fn!(i32, test_weeks, weeks);
        test_fn!(i32, test_months, months);
        test_fn!(i32, test_years, years);
    }
}
