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

impl<Left, Right> EqAll<(Right,)> for (Left,)
where
    Left: EqAll<Right>,
{
    type Output = <Left as EqAll<Right>>::Output;

    fn eq_all(self, rhs: (Right,)) -> Self::Output {
        self.0.eq_all(rhs.0)
    }
}

#[diesel_derives::__diesel_for_each_tuple(index_start = 1)]
impl<Left1, #[repeat] Left, Right1, #[repeat] Right> EqAll<(Right1, Right)> for (Left1, Left)
where
    Left1: EqAll<Right1>,
    (Left,): EqAll<(Right,)>,
{
    type Output =
        Grouped<And<<Left1 as EqAll<Right1>>::Output, <(Left,) as EqAll<(Right,)>>::Output>>;

    fn eq_all(self, rhs: (Right1, Right)) -> Self::Output {
        let new_lhs = #[repeat]
        (self.idx,);
        let new_rhs = #[repeat]
        (rhs.idx,);
        self.0.eq_all(rhs.0).and(new_lhs.eq_all(new_rhs))
    }
}
