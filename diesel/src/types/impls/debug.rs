use std::io::Write;
use std::error::Error;
use std::time::SystemTime;

use backend::Debug;
use types::*;

macro_rules! debug_to_sql {
    ($sql_type:ty, $ty:ty) => {
        impl ToSql<$sql_type, Debug> for $ty {
            fn to_sql<W: Write>(&self, _: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
                Ok(IsNull::No)
            }
        }
    };
}

debug_to_sql!(Bool, bool);
debug_to_sql!(Timestamp, SystemTime);

#[cfg(feature = "postgres")]
mod pg_impls {
    use super::*;
    use data_types::*;

    debug_to_sql!(Timestamp, PgTimestamp);
    debug_to_sql!(Timestamptz, PgTimestamp);
    debug_to_sql!(Date, PgDate);
    debug_to_sql!(Time, PgTime);
    debug_to_sql!(Interval, PgInterval);
}

#[cfg(feature = "deprecated-time")]
mod deprecated_time_impls {
    extern crate time;
    use super::*;
    use self::time::Timespec;

    debug_to_sql!(Timestamp, Timespec);
}

#[cfg(feature = "chrono")]
mod chrono_impls {
    extern crate chrono;
    use super::*;
    use self::chrono::{NaiveDateTime, NaiveTime, NaiveDate};
    #[cfg(feature = "postgres")]
    use self::chrono::{DateTime, TimeZone};

    debug_to_sql!(Timestamp, NaiveDateTime);
    debug_to_sql!(Time, NaiveTime);
    debug_to_sql!(Date, NaiveDate);

    #[cfg(feature = "postgres")]
    impl<TZ: TimeZone> ToSql<Timestamptz, Debug> for DateTime<TZ> {
        fn to_sql<W: Write>(&self, _: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
            Ok(IsNull::No)
        }
    }
}
