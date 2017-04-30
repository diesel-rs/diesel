use expression::Expression;
use expression::predicates::And;
use expression::expression_methods::*;
use types::Bool;

/// This method is used by `FindDsl` to work with tuples. Because we cannot
/// express this without specialization or overlapping impls, it is brute force
/// implemented on columns in the `column!` macro.
#[doc(hidden)]
pub trait EqAll<Rhs> {
    type Output: Expression<SqlType=Bool>;

    fn eq_all(self, rhs: Rhs) -> Self::Output;
}

macro_rules! ty_Output {
    ( $l:ident : $r:ident, ) => ( <$l as EqAll<$r>>::Output );
    ( $l:ident : $r:ident, $($oleft:ident : $oright:ident,)* ) => (
        And<<$l as EqAll<$r>>::Output, ty_Output! { $($oleft : $oright,)* }>
    )
}

macro_rules! chain_eq_all {
    ( $idx:tt : $l:tt : $r:tt, ) => (
        $l.$idx.eq_all($r.$idx)
    );
    ( $idx:tt : $l:tt : $r:tt, $($oidx:tt : $oleft:tt : $oright:tt,)* ) => (
        $l.$idx.eq_all($r.$idx).and(chain_eq_all! { $($oidx: $oleft : $oright,)* })
    )
}

macro_rules! impl_EqAll {
    ( $($idx:tt : $l:ident : $r:ident,)+ ) => (
        impl<$($l,)* $($r,)*> EqAll<($($r,)*)> for ($($l,)*)
            where $($l : EqAll<$r>,)*
        {
            type Output = ty_Output! { $($l : $r,)* };

            fn eq_all(self, rhs: ($($r,)*)) -> Self::Output {
                chain_eq_all! { $($idx : self : rhs,)* }
            }
        }
    )
}

impl_EqAll! { 0 : L1 : R1, 1 : L2 : R2, }
impl_EqAll! { 0 : L1 : R1, 1 : L2 : R2, 2 : L3 : R3, }
impl_EqAll! { 0 : L1 : R1, 1 : L2 : R2, 2 : L3 : R3, 3 : L4 : R4, }
