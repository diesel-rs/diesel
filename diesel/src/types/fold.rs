use types::{self, NotNull};

/// Represents SQL types which can be used with `SUM` and `AVG`
pub trait Foldable {
    /// The SQL type of `sum(this_type)`
    type Sum;
    /// The SQL type of `avg(this_type)`
    type Avg;
}

impl<T> Foldable for types::Nullable<T>
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
                type Sum = types::Nullable<$SumType>;
                type Avg = types::Nullable<$AvgType>;
            }
        )+
    }
}

foldable_impls! {
    types::SmallInt => (types::BigInt, types::Numeric),
    types::Integer => (types::BigInt, types::Numeric),
    types::BigInt => (types::Numeric, types::Numeric),

    types::Float => (types::Float, types::Double),
    types::Double => (types::Double, types::Double),
    types::Numeric => (types::Numeric, types::Numeric),

    types::Interval => (types::Interval, types::Interval),
}
