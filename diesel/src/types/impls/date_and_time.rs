#[cfg(feature="chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::*;
    use types::{Date, Time, Timestamp};

    expression_impls!(Date -> NaiveDate);
    expression_impls!(Time -> NaiveTime);
    expression_impls!(Timestamp -> NaiveDateTime);

    queryable_impls!(Date -> NaiveDate);
    queryable_impls!(Time -> NaiveTime);
    queryable_impls!(Timestamp -> NaiveDateTime);
}
