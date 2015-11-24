use super::{Expression, AsExpression};

pub type AsExpr<Item, TargetExpr> = <Item as AsExpression<
    <TargetExpr as Expression>::SqlType
>>::Expression;
