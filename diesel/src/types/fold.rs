use types::{self, NotNull};

pub trait Foldable {
    type Sum;
    type Avg;
}

impl<T> Foldable for types::Nullable<T> where
    T: Foldable + NotNull,
    T::Sum: NotNull,
    T::Avg: NotNull,
{
    type Sum = types::Nullable<T::Sum>;
    type Avg = types::Nullable<T::Avg>;
}

macro_rules! foldable_impls {
    ($($Source:ty => ($SumType:ty, $AvgType:ty)),+,) => {
        $(
            impl Foldable for $Source {
                type Sum = $SumType;
                type Avg = $AvgType;
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
