use std::ops::Mul;

use data_types::PgInterval;

/// A DSL added to `i64` and `f64` to construct PostgreSQL intervals of less
/// than 1 day.
///
/// The behavior of these methods when called on `NAN` or `Infinity` is
/// undefined.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// # use diesel::expression::dsl::*;
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #         created_at -> Timestamp,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = connection_no_data();
/// #     connection.execute("CREATE TABLE users (id serial primary key, name
/// #        varchar not null, created_at timestamp not null)").unwrap();
/// connection.execute("INSERT INTO users (name, created_at) VALUES
///     ('Sean', NOW()), ('Tess', NOW() - '5 minutes'::interval),
///     ('Jim', NOW() - '10 minutes'::interval)").unwrap();
///
/// let mut data = users
///     .select(name)
///     .filter(created_at.gt(now - 7.minutes()))
///     .load(&connection).unwrap();
/// assert_eq!(Some("Sean".to_string()), data.next());
/// assert_eq!(Some("Tess".to_string()), data.next());
/// assert_eq!(None, data.next());
/// # }
/// ```
pub trait MicroIntervalDsl: Sized + Mul<Self, Output=Self> {
    /// Returns a PgInterval representing `self` as microseconds
    fn microseconds(self) -> PgInterval;
    #[doc(hidden)]
    fn times(self, x: i32) -> Self;

    /// Returns a PgInterval representing `self` as milliseconds
    fn milliseconds(self) -> PgInterval {
        (self.times(1000)).microseconds()
    }

    /// Returns a PgInterval representing `self` as seconds
    fn seconds(self) -> PgInterval {
        (self.times(1000)).milliseconds()
    }

    /// Returns a PgInterval representing `self` as minutes
    fn minutes(self) -> PgInterval {
        (self.times(60)).seconds()
    }

    /// Returns a PgInterval representing `self` as hours
    fn hours(self) -> PgInterval {
        (self.times(60)).minutes()
    }

    /// Identical to `microseconds`
    fn microsecond(self) -> PgInterval {
        self.microseconds()
    }

    /// Identical to `milliseconds`
    fn millisecond(self) -> PgInterval {
        self.milliseconds()
    }

    /// Identical to `seconds`
    fn second(self) -> PgInterval {
        self.seconds()
    }

    /// Identical to `minutes`
    fn minute(self) -> PgInterval {
        self.minutes()
    }

    /// Identical to `hours`
    fn hour(self) -> PgInterval {
        self.hours()
    }
}

/// A DSL added to `i32` and `f64` to construct PostgreSQL intervals of greater
/// than 1 day.
///
/// The behavior of these methods when called on `NAN` or `Infinity` is
/// undefined.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// # use diesel::expression::dsl::*;
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #         created_at -> Timestamp,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = connection_no_data();
/// #     connection.execute("CREATE TABLE users (id serial primary key, name
/// #        varchar not null, created_at timestamp not null)").unwrap();
/// connection.execute("INSERT INTO users (name, created_at) VALUES
///     ('Sean', NOW()), ('Tess', NOW() - '5 days'::interval),
///     ('Jim', NOW() - '10 days'::interval)").unwrap();
///
/// let mut data = users
///     .select(name)
///     .filter(created_at.gt(now - 7.days()))
///     .load(&connection).unwrap();
/// assert_eq!(Some("Sean".to_string()), data.next());
/// assert_eq!(Some("Tess".to_string()), data.next());
/// assert_eq!(None, data.next());
/// # }
/// ```
pub trait DayAndMonthIntervalDsl: Sized + Mul<Self, Output=Self>  {
    /// Returns a PgInterval representing `self` in days
    fn days(self) -> PgInterval;
    /// Returns a PgInterval representing `self` in monhts
    fn months(self) -> PgInterval;
    #[doc(hidden)]
    fn times(self, x: i32) -> Self;

    /// Returns a PgInterval representing `self` in weeks
    ///
    /// Note: When called on a high precision float, the returned interval may
    /// be 1 microsecond different than the equivalent string passed to
    /// PostgreSQL.
    fn weeks(self) -> PgInterval {
        (self.times(7)).days()
    }

    /// Returns a PgInterval representing `self` in weeks
    ///
    /// Note: When called on a float, this method will mimic the behavior of
    /// PostgreSQL's interval parsing, and will ignore units smaller than
    /// months.
    ///
    /// ```rust
    /// # use diesel::expression::dsl::*;
    /// assert_eq!(1.08.years(), 1.year());
    /// assert_eq!(1.09.years(), 1.year() + 1.month());
    /// ```
    fn years(self) -> PgInterval {
        (self.times(12)).months()
    }

    /// Identical to `days`
    fn day(self) -> PgInterval {
        self.days()
    }

    /// Identical to `weeks`
    fn week(self) -> PgInterval {
        self.weeks()
    }

    /// Identical to `months`
    fn month(self) -> PgInterval {
        self.months()
    }

    /// Identical to `years`
    fn year(self) -> PgInterval {
        self.years()
    }
}

impl MicroIntervalDsl for i64 {
    fn microseconds(self) -> PgInterval {
        PgInterval::from_microseconds(self)
    }

    fn times(self, x: i32) -> i64 {
        self * x as i64
    }
}

impl MicroIntervalDsl for f64 {
    fn microseconds(self) -> PgInterval {
        (self.round() as i64).microseconds()
    }

    fn times(self, x: i32) -> f64 {
        self * x as f64
    }
}

impl DayAndMonthIntervalDsl for i32 {
    fn days(self) -> PgInterval {
        PgInterval::from_days(self)
    }

    fn months(self) -> PgInterval {
        PgInterval::from_months(self)
    }

    fn times(self, x: i32) -> i32 {
        self * x as i32
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

    fn times(self, x: i32) -> f64 {
        self * x as f64
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;
    extern crate quickcheck;

    use self::quickcheck::quickcheck;
    use self::dotenv::dotenv;

    use ::{types, select};
    use connection::Connection;
    use data_types::PgInterval;
    use expression::dsl::sql;
    use prelude::*;
    use super::*;

    macro_rules! test_fn {
        ($tpe:ty, $test_name:ident, $units:ident) => {
            fn $test_name(val: $tpe) -> bool {
                dotenv().ok();

                let connection_url = ::std::env::var("DATABASE_URL").ok()
                    .expect("DATABASE_URL must be set in order to run tests");
                let connection = Connection::establish(&connection_url).unwrap();

                let sql_str = format!(concat!("'{} ", stringify!($units), "'::interval"), val);
                let query = select(sql::<types::Interval>(&sql_str));
                let res = query.get_result::<PgInterval>(&connection).unwrap();
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
