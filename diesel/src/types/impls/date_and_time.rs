debug_to_sql!(::types::Timestamp, ::std::time::SystemTime);

#[cfg(feature="chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::{NaiveDateTime, NaiveDate, NaiveTime};

    debug_to_sql!(::types::Timestamp, NaiveDateTime);
    debug_to_sql!(::types::Time, NaiveTime);
    debug_to_sql!(::types::Date, NaiveDate);
}
