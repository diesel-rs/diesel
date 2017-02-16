debug_to_sql!(::types::Timestamp, ::std::time::SystemTime);

#[cfg(feature="chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::*;

    debug_to_sql!(::types::Timestamp, NaiveDateTime);
    debug_to_sql!(::types::Time, NaiveTime);
    debug_to_sql!(::types::Date, NaiveDate);

    expression_impls! {
        Date -> NaiveDate,
        Time -> NaiveTime,
        Timestamp -> NaiveDateTime,
    }

    queryable_impls! {
        Date -> NaiveDate,
        Time -> NaiveTime,
        Timestamp -> NaiveDateTime,
    }
}
