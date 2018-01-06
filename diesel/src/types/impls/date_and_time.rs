#![allow(dead_code)]

use std::time::SystemTime;

#[derive(FromSqlRow, AsExpression)]
#[diesel(foreign_derive)]
#[sql_type = "::types::Timestamp"]
struct SystemTimeProxy(SystemTime);

#[cfg(feature = "chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::*;
    use types::{Date, Time, Timestamp};

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Date"]
    struct NaiveDateProxy(NaiveDate);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Time"]
    struct NaiveTimeProxy(NaiveTime);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Timestamp"]
    #[cfg_attr(feature = "postgres", sql_type = "::types::Timestamptz")]
    #[cfg_attr(feature = "mysql", sql_type = "::types::Datetime")]
    struct NaiveDateTimeProxy(NaiveDateTime);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "postgres", sql_type = "::types::Timestamptz")]
    struct DateTimeProxy<Tz: TimeZone>(DateTime<Tz>);
}
