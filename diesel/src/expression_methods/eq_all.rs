use expression::Expression;
use expression::operators::And;
use expression_methods::*;
use types::Bool;

/// This method is used by `FindDsl` to work with tuples. Because we cannot
/// express this without specialization or overlapping impls, it is brute force
/// implemented on columns in the `column!` macro.
#[doc(hidden)]
pub trait EqAll<Rhs> {
    type Output: Expression<SqlType=Bool>;

    fn eq_all(self, rhs: Rhs) -> Self::Output;
}

// FIXME: This is much easier to represent with a macro once macro types are stable
// which appears to be slated for 1.13
impl<L1, L2, R1, R2> EqAll<(R1, R2)> for (L1, L2) where
    L1: EqAll<R1>,
    L2: EqAll<R2>,
{
    type Output = And<<L1 as EqAll<R1>>::Output, <L2 as EqAll<R2>>::Output>;

    fn eq_all(self, rhs: (R1, R2)) -> Self::Output {
        self.0.eq_all(rhs.0).and(self.1.eq_all(rhs.1))
    }
}

impl<L1, L2, L3, R1, R2, R3> EqAll<(R1, R2, R3)> for (L1, L2, L3) where
    L1: EqAll<R1>,
    L2: EqAll<R2>,
    L3: EqAll<R3>,
{
    type Output = And<<L1 as EqAll<R1>>::Output, And<<L2 as EqAll<R2>>::Output, <L3 as EqAll<R3>>::Output>>;

    fn eq_all(self, rhs: (R1, R2, R3)) -> Self::Output {
        self.0.eq_all(rhs.0).and(
            self.1.eq_all(rhs.1).and(self.2.eq_all(rhs.2)))
    }
}

impl<L1, L2, L3, L4, R1, R2, R3, R4> EqAll<(R1, R2, R3, R4)> for (L1, L2, L3, L4) where
    L1: EqAll<R1>,
    L2: EqAll<R2>,
    L3: EqAll<R3>,
    L4: EqAll<R4>,
{
    type Output = And<<L1 as EqAll<R1>>::Output, And<<L2 as EqAll<R2>>::Output, And<<L3 as EqAll<R3>>::Output, <L4 as EqAll<R4>>::Output>>>;

    fn eq_all(self, rhs: (R1, R2, R3, R4)) -> Self::Output {
        self.0.eq_all(rhs.0).and(
            self.1.eq_all(rhs.1).and(
            self.2.eq_all(rhs.2).and(
            self.3.eq_all(rhs.3)
            )))
    }
}
