use crate::expression::grouped::Grouped;
use crate::expression::operators::And;
use crate::expression::Expression;
use crate::expression_methods::*;
use crate::sql_types::Bool;

/// This method is used by `FindDsl` to work with tuples. Because we cannot
/// express this without specialization or overlapping impls, it is brute force
/// implemented on columns in the `column!` macro.
#[doc(hidden)]
pub trait EqAll<Rhs> {
    type Output: Expression<SqlType = Bool>;

    fn eq_all(self, rhs: Rhs) -> Self::Output;
}

macro_rules! impl_eq_all {
    // General case for 2+ elements
    (
        ($Left1:ident, $($Left:ident,)+)
        ($Right1:ident, $($Right:ident,)+)
    ) => {
        #[allow(non_snake_case)]
        impl<$Left1, $($Left,)+ $Right1, $($Right,)+>
            EqAll<($Right1, $($Right,)+)> for ($Left1, $($Left,)+)
        where
            $Left1: EqAll<$Right1>,
            ($($Left,)+): EqAll<($($Right,)+)>,
        {
            type Output = Grouped<And<
                <$Left1 as EqAll<$Right1>>::Output,
                <($($Left,)+) as EqAll<($($Right,)+)>>::Output,
            >>;

            fn eq_all(self, rhs: ($Right1, $($Right,)+)) -> Self::Output {
                let ($Left1, $($Left,)+) = self;
                let ($Right1, $($Right,)+) = rhs;
                $Left1.eq_all($Right1).and(($($Left,)+).eq_all(($($Right,)+)))
            }
        }
    };

    // Special case for 1 element
    (
        ($Left:ident,) ($Right:ident,)
    ) => {
        impl<$Left, $Right> EqAll<($Right,)> for ($Left,)
        where
            $Left: EqAll<$Right>,
        {
            type Output = <$Left as EqAll<$Right>>::Output;

            fn eq_all(self, rhs: ($Right,)) -> Self::Output {
                self.0.eq_all(rhs.0)
            }
        }
    };
}

macro_rules! impl_eq_all_for_all_tuples {
    ($(
        $unused1:tt {
            $($unused2:tt -> $Left:ident, $Right:ident, $unused3:tt,)+
        }
    )+) => {
        $(
            impl_eq_all!(($($Left,)+) ($($Right,)+));
        )+
    };
}

diesel_derives::__diesel_for_each_tuple!(impl_eq_all_for_all_tuples);
