use types::{self, NotNull};

/// Marker trait for types which can be folded for a sum.
pub trait Foldable {
    type Sum;
    type Avg;
}

impl<T> Foldable for types::Nullable<T> where
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
