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

impl<L1, L2, LTail, R1, R2, RTail> EqAll<Cons<R1, Cons<R2, RTail>>>
    for Cons<L1, Cons<L2, LTail>> where
        L1: EqAll<R1>,
        Cons<L2, LTail>: EqAll<Cons<R2, RTail>>,
{
    type Output = And<<L1 as EqAll<R1>>::Output, <Cons<L2, LTail> as EqAll<Cons<R2, RTail>>>::Output>;

    fn eq_all(self, rhs: Cons<R1, Cons<R2, RTail>>) -> Self::Output {
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
