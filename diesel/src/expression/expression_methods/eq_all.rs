use expression::Expression;
use expression::expression_methods::*;
use expression::predicates::And;
use hlist::*;
use types::Bool;

/// This method is used by `FindDsl` to work with hlists. Because we cannot
/// express this without specialization or overlapping impls, it is brute force
/// implemented on columns in the `column!` macro.
#[doc(hidden)]
pub trait EqAll<Rhs> {
    type Output: Expression<SqlType=Bool>;

    fn eq_all(self, rhs: Rhs) -> Self::Output;
}

impl<LHead, LTail, RHead, RTail> EqAll<Cons<RHead, RTail>>
    for Cons<LHead, LTail> where
        LHead: EqAll<RHead>,
        LTail: EqAll<RTail>,
{
    type Output = And<<LHead as EqAll<RHead>>::Output, <LTail as EqAll<RTail>>::Output>;

    fn eq_all(self, rhs: Cons<RHead, RTail>) -> Self::Output {
        self.0.eq_all(rhs.0).and(self.1.eq_all(rhs.1))
    }
}

impl<Left, Right> EqAll<Cons<Right, Nil>> for Cons<Left, Nil> where
    Left: EqAll<Right>,
{
    type Output = <Left as EqAll<Right>>::Output;

    fn eq_all(self, rhs: Cons<Right, Nil>) -> Self::Output {
        self.0.eq_all(rhs.0)
    }
}
