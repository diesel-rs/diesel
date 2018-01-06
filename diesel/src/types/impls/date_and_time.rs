#![allow(dead_code)]

use std::time::SystemTime;

#[derive(FromSqlRow)]
#[diesel(foreign_derive)]
struct SystemTimeProxy(SystemTime);

#[cfg(feature = "chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::*;
    use types::{Date, Time, Timestamp};

    expression_impls!(Date -> NaiveDate);
    expression_impls!(Time -> NaiveTime);
    expression_impls!(Timestamp -> NaiveDateTime);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct NaiveDateProxy(NaiveDate);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct NaiveTimeProxy(NaiveTime);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct NaiveDateTimeProxy(NaiveDateTime);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct DateTimeProxy<Tz: TimeZone>(DateTime<Tz>);
}
