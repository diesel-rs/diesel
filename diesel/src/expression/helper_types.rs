//! The types in this module are all shorthand for `PredicateType<Lhs,
//! AsExpr<Rhs, Lhs>>`. Since we often need to return concrete types, instead of
//! a boxed trait object, these can be useful for writing concise return types.
use super::{AsExpression, Expression};
use super::grouped::Grouped;
use sql_types;

/// The SQL type of an expression
pub type SqlTypeOf<Expr> = <Expr as Expression>::SqlType;

/// The type of `Item` when converted to an expression with the same type as `TargetExpr`
pub type AsExpr<Item, TargetExpr> = AsExprOf<Item, SqlTypeOf<TargetExpr>>;

/// The type of `Item` when converted to an expression of `Type`
pub type AsExprOf<Item, Type> = <Item as AsExpression<Type>>::Expression;

/// The return type of `lhs.eq(rhs)`
pub type Eq<Lhs, Rhs> = super::operators::Eq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of `lhs.ne(rhs)`
pub type NotEq<Lhs, Rhs> = super::operators::NotEq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of `lhs.gt(rhs)`
pub type Gt<Lhs, Rhs> = super::operators::Gt<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of `lhs.ge(rhs)`
pub type GtEq<Lhs, Rhs> = super::operators::GtEq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of `lhs.lt(rhs)`
pub type Lt<Lhs, Rhs> = super::operators::Lt<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of `lhs.le(rhs)`
pub type LtEq<Lhs, Rhs> = super::operators::LtEq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of `lhs.and(rhs)`
pub type And<Lhs, Rhs> = super::operators::And<Lhs, AsExprOf<Rhs, sql_types::Bool>>;

/// The return type of `lhs.or(rhs)`
pub type Or<Lhs, Rhs> = Grouped<super::operators::Or<Lhs, AsExprOf<Rhs, types::Bool>>>;

/// The return type of `lhs.like(rhs)`
pub type Like<Lhs, Rhs> = super::operators::Like<Lhs, AsExprOf<Rhs, sql_types::VarChar>>;

/// The return type of `lhs.not_like(rhs)`
pub type NotLike<Lhs, Rhs> = super::operators::NotLike<Lhs, AsExprOf<Rhs, sql_types::VarChar>>;

/// The return type of `lhs.between(lower, upper)`
pub type Between<Lhs, Lower, Upper> = super::operators::Between<
    Lhs,
    super::operators::And<AsExpr<Lower, Lhs>, AsExpr<Upper, Lhs>>,
>;
/// The return type of `lhs.not_between(lower, upper)`
pub type NotBetween<Lhs, Lower, Upper> = super::operators::NotBetween<
    Lhs,
    super::operators::And<AsExpr<Lower, Lhs>, AsExpr<Upper, Lhs>>,
>;
/// The return type of `not(expr)`
pub type Not<Expr> = super::operators::Not<Grouped<AsExprOf<Expr, sql_types::Bool>>>;

#[doc(inline)]
pub use super::operators::{Asc, Desc, IsNotNull, IsNull};
#[doc(inline)]
pub use super::array_comparison::EqAny;
