use crate::dsl::AsExpr;
use crate::expression::grouped::Grouped;

/// The return type of `lhs.is(rhs)`.
pub type Is<Lhs, Rhs> = Grouped<super::operators::Is<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.is_not(rhs)`.
pub type IsNot<Lhs, Rhs> = Grouped<super::operators::IsNot<Lhs, AsExpr<Rhs, Lhs>>>;
