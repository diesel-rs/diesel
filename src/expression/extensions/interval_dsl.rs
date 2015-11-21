use std::ops::Mul;

use types::structs::PgInterval;

pub trait MicroIntervalDsl: Sized + From<i32> + Mul<Self, Output=Self> {
    fn microseconds(self) -> PgInterval;

    fn milliseconds(self) -> PgInterval {
        (self * 1000.into()).microseconds()
    }

    fn seconds(self) -> PgInterval {
        (self * 1000.into()).milliseconds()
    }

    fn minutes(self) -> PgInterval {
        (self * 60.into()).seconds()
    }

    fn hours(self) -> PgInterval {
        (self * 60.into()).minutes()
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

pub trait DayAndMonthIntervalDsl: Sized + From<i32> + Mul<Self, Output=Self>  {
    fn days(self) -> PgInterval;
    fn months(self) -> PgInterval;

    fn weeks(self) -> PgInterval {
        (self * 7.into()).days()
    }

    fn years(self) -> PgInterval {
        (self * 12.into()).months()
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
        PgInterval::from_microseconds(self)
    }
}

impl MicroIntervalDsl for f64 {
    fn microseconds(self) -> PgInterval {
        (self.round() as i64).microseconds()
    }
}

impl DayAndMonthIntervalDsl for i32 {
    fn days(self) -> PgInterval {
        PgInterval::from_days(self)
    }

    fn months(self) -> PgInterval {
        PgInterval::from_months(self)
    }
}

impl DayAndMonthIntervalDsl for f64 {
    fn days(self) -> PgInterval {
        let fractional_days = (self.fract() * 86_400.0).seconds();
        PgInterval::from_days(self.trunc() as i32) + fractional_days
    }

    fn months(self) -> PgInterval {
        let fractional_months = (self.fract() * 30.0).days();
        PgInterval::from_months(self.trunc() as i32) + fractional_months
    }

    fn years(self) -> PgInterval {
        ((self * 12.0).trunc() as i32).months()
    }
}

#[cfg(test)]
mod tests {
    extern crate quickcheck;
    use self::quickcheck::quickcheck;
    use super::*;
    use connection::Connection;
    use types;
    use types::structs::PgInterval;

    macro_rules! test_fn {
        ($tpe:ty, $test_name:ident, $units:ident) => {
            fn $test_name(val: $tpe) -> bool {
                let connection_url = ::std::env::var("DATABASE_URL").ok()
                    .expect("DATABASE_URL must be set in order to run tests");
                let connection = Connection::establish(&connection_url).unwrap();

                let query = format!(concat!("SELECT '{} ", stringify!($units), "'::interval"), val);
                let res: PgInterval = connection.query_sql::<types::Interval, _>(&query)
                    .unwrap().nth(0).unwrap();
                let val = val.$units();
                val.months == res.months &&
                    val.days == res.days &&
                    (val.microseconds - res.microseconds).abs() <= 1
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
    fn micro_intervals_match_pg_values_f64() {
        test_fn!(f64, test_microseconds, microseconds);
        test_fn!(f64, test_milliseconds, milliseconds);
        test_fn!(f64, test_seconds, seconds);
        test_fn!(f64, test_minutes, minutes);
        test_fn!(f64, test_hours, hours);
    }

    #[test]
    fn day_and_month_intervals_match_pg_values_i32() {
        test_fn!(i32, test_days, days);
        test_fn!(i32, test_weeks, weeks);
        test_fn!(i32, test_months, months);
        test_fn!(i32, test_years, years);
    }

    #[test]
    fn day_and_month_intervals_match_pg_values_f64() {
        test_fn!(f64, test_days, days);
        test_fn!(f64, test_weeks, weeks);
        test_fn!(f64, test_months, months);
        test_fn!(f64, test_years, years);
    }
}
