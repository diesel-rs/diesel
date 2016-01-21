//! The types in this module are all shorthand for `PredicateType<Lhs,
//! AsExpr<Rhs, Lhs>>`. Since we often need to return concrete types, instead of
//! a boxed trait object, these can be useful for writing concise return types.
use super::{Expression, AsExpression};
use types;

pub type AsExpr<Item, TargetExpr> = <Item as AsExpression<
    <TargetExpr as Expression>::SqlType
>>::Expression;

macro_rules! gen_helper_type {
    ($name:ident) => {
        pub type $name<Lhs, Rhs> = super::predicates::$name<Lhs, AsExpr<Rhs, Lhs>>;
    };

    ($name:ident, $tpe:ident) => {
        pub type $name<Lhs, Rhs> = super::predicates::$name<
            Lhs,
            <Rhs as AsExpression<types::$tpe>>::Expression,
        >;
    }
}

gen_helper_type!(Eq);
gen_helper_type!(NotEq);
gen_helper_type!(Gt);
gen_helper_type!(GtEq);
gen_helper_type!(Lt);
gen_helper_type!(LtEq);
gen_helper_type!(And, Bool);
gen_helper_type!(Like, VarChar);
gen_helper_type!(NotLike, VarChar);

pub type Between<Lhs, Rhs> = super::predicates::Between<Lhs,
    super::predicates::And<AsExpr<Rhs, Lhs>, AsExpr<Rhs, Lhs>>>;
pub type NotBetween<Lhs, Rhs> = super::predicates::NotBetween<Lhs,
    super::predicates::And<AsExpr<Rhs, Lhs>, AsExpr<Rhs, Lhs>>>;

pub use super::predicates::{IsNull, IsNotNull, Asc, Desc};
