#[cfg(feature="chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::*;

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
