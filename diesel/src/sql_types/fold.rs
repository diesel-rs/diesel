use sql_types::{self, NotNull};

/// Represents SQL types which can be used with `SUM` and `AVG`
pub trait Foldable {
    /// The SQL type of `sum(this_type)`
    type Sum;
    /// The SQL type of `avg(this_type)`
    type Avg;
}

impl<T> Foldable for sql_types::Nullable<T>
where
    T: Foldable + NotNull,
{
    type Sum = T::Sum;
    type Avg = T::Avg;
}

macro_rules! foldable_impls {
    ($($Source:ty => ($SumType:ty, $AvgType:ty)),+,) => {
        $(
            impl Foldable for $Source {
                type Sum = sql_types::Nullable<$SumType>;
                type Avg = sql_types::Nullable<$AvgType>;
            }
        )+
    }
}

foldable_impls! {
    sql_types::SmallInt => (sql_types::BigInt, sql_types::Numeric),
    sql_types::Integer => (sql_types::BigInt, sql_types::Numeric),
    sql_types::BigInt => (sql_types::Numeric, sql_types::Numeric),

    sql_types::Float => (sql_types::Float, sql_types::Double),
    sql_types::Double => (sql_types::Double, sql_types::Double),
    sql_types::Numeric => (sql_types::Numeric, sql_types::Numeric),

    sql_types::Interval => (sql_types::Interval, sql_types::Interval),
}

#[cfg(feature = "mysql")]
foldable_impls! {
    sql_types::Unsigned<sql_types::SmallInt> => (sql_types::Unsigned<sql_types::BigInt>, sql_types::Numeric),
    sql_types::Unsigned<sql_types::Integer> => (sql_types::Unsigned<sql_types::BigInt>, sql_types::Numeric),
    sql_types::Unsigned<sql_types::BigInt> => (sql_types::Numeric, sql_types::Numeric),
}
