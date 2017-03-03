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

impl<L1, L2, LTail, R1, R2, RTail> EqAll<(R1, R2, ...RTail)>
    for (L1, L2, ...LTail) where
        L1: EqAll<R1>,
        (L2, ...LTail): EqAll<(R2, ...RTail)>,
        LTail: Tuple,
        RTail: Tuple,
{
    type Output = And<<L1 as EqAll<R1>>::Output, <(L1, ...LTail) as EqAll<(R1, ...RTail)>>::Output>;

    fn eq_all(self, rhs: (R1, R2, ...RTail)) -> Self::Output {
        let (lhead, ...ltail) = self;
        let (rhead, ...rtail) = rhs;
        lhead.eq_all(rhead).and(ltail.eq_all(rtail))
    }
}

impl<Left, Right> EqAll<(Right,)> for Cons<(Left,)> where
    Left: EqAll<Right>,
{
    type Output = <Left as EqAll<Right>>::Output;

    fn eq_all(self, rhs: (Right,)) -> Self::Output {
        self.0.eq_all(rhs.0)
    }
}
