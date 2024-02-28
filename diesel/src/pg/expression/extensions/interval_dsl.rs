use std::ops::Mul;

use crate::data_types::PgInterval;

/// A DSL added to integers and `f64` to construct PostgreSQL intervals.
///
/// The behavior of these methods when called on `NAN` or `Infinity`, or when
/// overflow occurs is unspecified.
///
/// # Examples
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # use diesel::dsl::*;
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
/// #     let connection = &mut connection_no_data();
/// #     diesel::sql_query("CREATE TABLE users (id serial primary key, name
/// #        varchar not null, created_at timestamp not null)")
/// #     .execute(connection)
/// #     .unwrap();
/// diesel::sql_query("INSERT INTO users (name, created_at) VALUES
///     ('Sean', NOW()), ('Tess', NOW() - '5 minutes'::interval),
///     ('Jim', NOW() - '10 minutes'::interval)")
///     .execute(connection)
///     .unwrap();
///
/// let mut data: Vec<String> = users
///     .select(name)
///     .filter(created_at.gt(now - 7.minutes()))
///     .load(connection).unwrap();
/// assert_eq!(2, data.len());
/// assert_eq!("Sean".to_string(), data[0]);
/// assert_eq!("Tess".to_string(), data[1]);
/// # }
/// ```
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # use diesel::dsl::*;
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
/// #     let connection = &mut connection_no_data();
/// #     diesel::sql_query("CREATE TABLE users (id serial primary key, name
/// #        varchar not null, created_at timestamp not null)")
/// #     .execute(connection)
/// #     .unwrap();
/// diesel::sql_query("INSERT INTO users (name, created_at) VALUES
///     ('Sean', NOW()), ('Tess', NOW() - '5 days'::interval),
///     ('Jim', NOW() - '10 days'::interval)")
///     .execute(connection)
///     .unwrap();
///
/// let mut data: Vec<String> = users
///     .select(name)
///     .filter(created_at.gt(now - 7.days()))
///     .load(connection).unwrap();
/// assert_eq!(2, data.len());
/// assert_eq!("Sean".to_string(), data[0]);
/// assert_eq!("Tess".to_string(), data[1]);
/// # }
/// ```
#[cfg(feature = "postgres_backend")]
pub trait IntervalDsl: Sized + From<i32> + Mul<Self, Output = Self> {
    /// Returns a PgInterval representing `self` as microseconds
    fn microseconds(self) -> PgInterval;
    /// Returns a PgInterval representing `self` in days
    fn days(self) -> PgInterval;
    /// Returns a PgInterval representing `self` in months
    fn months(self) -> PgInterval;

    /// Returns a PgInterval representing `self` as milliseconds
    fn milliseconds(self) -> PgInterval {
        (self * 1000.into()).microseconds()
    }

    /// Returns a PgInterval representing `self` as seconds
    fn seconds(self) -> PgInterval {
        (self * 1000.into()).milliseconds()
    }

    /// Returns a PgInterval representing `self` as minutes
    fn minutes(self) -> PgInterval {
        (self * 60.into()).seconds()
    }

    /// Returns a PgInterval representing `self` as hours
    fn hours(self) -> PgInterval {
        (self * 60.into()).minutes()
    }

    /// Returns a PgInterval representing `self` in weeks
    ///
    /// Note: When called on a high precision float, the returned interval may
    /// be 1 microsecond different than the equivalent string passed to
    /// PostgreSQL.
    fn weeks(self) -> PgInterval {
        (self * 7.into()).days()
    }

    /// Returns a PgInterval representing `self` in weeks
    ///
    /// Note: When called on a float, this method will mimic the behavior of
    /// PostgreSQL's interval parsing, and will ignore units smaller than
    /// months.
    ///
    /// ```rust
    /// # use diesel::dsl::*;
    /// assert_eq!(1.08.years(), 1.year());
    /// assert_eq!(1.09.years(), 1.year() + 1.month());
    /// ```
    fn years(self) -> PgInterval {
        (self * 12.into()).months()
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

impl IntervalDsl for i32 {
    fn microseconds(self) -> PgInterval {
        i64::from(self).microseconds()
    }

    fn days(self) -> PgInterval {
        PgInterval::from_days(self)
    }

    fn months(self) -> PgInterval {
        PgInterval::from_months(self)
    }

    fn milliseconds(self) -> PgInterval {
        i64::from(self).milliseconds()
    }

    fn seconds(self) -> PgInterval {
        i64::from(self).seconds()
    }

    fn minutes(self) -> PgInterval {
        i64::from(self).minutes()
    }

    fn hours(self) -> PgInterval {
        i64::from(self).hours()
    }
}

impl IntervalDsl for i64 {
    fn microseconds(self) -> PgInterval {
        PgInterval::from_microseconds(self)
    }

    fn days(self) -> PgInterval {
        (self as i32).days()
    }

    fn months(self) -> PgInterval {
        (self as i32).months()
    }
}

impl IntervalDsl for f64 {
    fn microseconds(self) -> PgInterval {
        (self.round() as i64).microseconds()
    }

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
// those macros define nested function
// that's fine for this test code
#[allow(clippy::items_after_statements)]
mod tests {
    extern crate dotenvy;
    extern crate quickcheck;

    use self::quickcheck::quickcheck;

    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::test_helpers::*;
    use crate::{select, sql_types};

    macro_rules! test_fn {
        ($tpe:ty, $test_name:ident, $units: ident, $max_range: expr) => {
            test_fn!($tpe, $test_name, $units, $max_range, 1);
        };
        ($tpe:ty, $test_name:ident, $units:ident, $max_range: expr, $max_diff: expr) => {
            fn $test_name(val: $tpe) -> bool {
                if val > $max_range || val < (-1 as $tpe) * $max_range || (val as f64).is_nan() {
                    return true;
                }
                let conn = &mut pg_connection();
                let sql_str = format!(concat!("'{} ", stringify!($units), "'::interval"), val);
                let query = select(sql::<sql_types::Interval>(&sql_str));
                let value = val.$units();
                query
                    .get_result::<PgInterval>(conn)
                    .map(|res| {
                        value.months == res.months
                            && value.days == res.days
                            && (value.microseconds - res.microseconds).abs() <= $max_diff
                    })
                    .unwrap_or(false)
            }

            quickcheck($test_name as fn($tpe) -> bool);
        };
    }

    #[test]
    fn intervals_match_pg_values_i32() {
        test_fn!(i32, test_microseconds, microseconds, i32::MAX);
        test_fn!(i32, test_milliseconds, milliseconds, i32::MAX);
        test_fn!(i32, test_seconds, seconds, i32::MAX);
        test_fn!(i32, test_minutes, minutes, i32::MAX);
        test_fn!(i32, test_hours, hours, i32::MAX);
        test_fn!(i32, test_days, days, i32::MAX);
        test_fn!(i32, test_weeks, weeks, i32::MAX / 7);
        test_fn!(i32, test_months, months, i32::MAX);
        test_fn!(i32, test_years, years, i32::MAX / 12);
    }

    #[test]
    fn intervals_match_pg_values_i64() {
        // postgres does not really support intervals with more than i32::MAX microseconds
        // https://www.postgresql.org/message-id/20140126025049.GL9750@momjian.us
        test_fn!(i64, test_microseconds, microseconds, i32::MAX as i64);
        test_fn!(i64, test_milliseconds, milliseconds, i32::MAX as i64);
        test_fn!(i64, test_seconds, seconds, i32::MAX as i64);
        test_fn!(i64, test_minutes, minutes, i32::MAX as i64);
        test_fn!(i64, test_hours, hours, i32::MAX as i64);
        test_fn!(i64, test_days, days, i32::MAX as i64);
        test_fn!(i64, test_weeks, weeks, (i32::MAX / 7) as i64);
        test_fn!(i64, test_months, months, i32::MAX as i64);
        test_fn!(i64, test_years, years, (i32::MAX / 12) as i64);
    }

    #[test]
    fn intervals_match_pg_values_f64() {
        const MAX_DIFF: i64 = 1_000_000;
        // postgres does not really support intervals with more than i32::MAX microseconds
        // https://www.postgresql.org/message-id/20140126025049.GL9750@momjian.us
        test_fn!(
            f64,
            test_microseconds,
            microseconds,
            i32::MAX as f64,
            MAX_DIFF
        );
        test_fn!(
            f64,
            test_milliseconds,
            milliseconds,
            i32::MAX as f64,
            MAX_DIFF
        );
        test_fn!(f64, test_seconds, seconds, i32::MAX as f64, MAX_DIFF);
        test_fn!(f64, test_minutes, minutes, i32::MAX as f64, MAX_DIFF);
        test_fn!(f64, test_hours, hours, i32::MAX as f64, MAX_DIFF);
        test_fn!(f64, test_days, days, i32::MAX as f64, MAX_DIFF);
        test_fn!(f64, test_weeks, weeks, (i32::MAX / 7) as f64, MAX_DIFF);
        test_fn!(f64, test_months, months, i32::MAX as f64, MAX_DIFF);
        test_fn!(f64, test_years, years, (i32::MAX / 12) as f64, MAX_DIFF);
    }
}
