//! The types in this module are all shorthand for `PredicateType<Lhs,
//! AsExpr<Rhs, Lhs>>`. Since we often need to return concrete types, instead of
//! a boxed trait object, these can be useful for writing concise return types.
use super::array_comparison::{AsInExpression, In, NotIn};
use super::grouped::Grouped;
use super::{AsExpression, Expression};
use crate::sql_types;

/// The SQL type of an expression
pub type SqlTypeOf<Expr> = <Expr as Expression>::SqlType;

/// The type of `Item` when converted to an expression with the same type as `TargetExpr`
pub type AsExpr<Item, TargetExpr> = AsExprOf<Item, SqlTypeOf<TargetExpr>>;

/// The type of `Item` when converted to an expression of `Type`
pub type AsExprOf<Item, Type> = <Item as AsExpression<Type>>::Expression;

/// The return type of
/// [`lhs.eq(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.eq)
pub type Eq<Lhs, Rhs> = super::operators::Eq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of
/// [`lhs.ne(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.ne)
pub type NotEq<Lhs, Rhs> = super::operators::NotEq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of
/// [`lhs.eq_any(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.eq_any)
pub type EqAny<Lhs, Rhs> = In<Lhs, <Rhs as AsInExpression<SqlTypeOf<Lhs>>>::InExpression>;

/// The return type of
/// [`lhs.ne_any(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.ne_any)
pub type NeAny<Lhs, Rhs> = NotIn<Lhs, <Rhs as AsInExpression<SqlTypeOf<Lhs>>>::InExpression>;

/// The return type of
/// [`expr.is_null()`](../expression_methods/trait.ExpressionMethods.html#method.is_null)
pub type IsNull<Expr> = super::operators::IsNull<Expr>;

/// The return type of
/// [`expr.is_not_null()`](../expression_methods/trait.ExpressionMethods.html#method.is_not_null)
pub type IsNotNull<Expr> = super::operators::IsNotNull<Expr>;

/// The return type of
/// [`lhs.gt(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.gt)
pub type Gt<Lhs, Rhs> = super::operators::Gt<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of
/// [`lhs.ge(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.ge)
pub type GtEq<Lhs, Rhs> = super::operators::GtEq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of
/// [`lhs.lt(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.lt)
pub type Lt<Lhs, Rhs> = super::operators::Lt<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of
/// [`lhs.le(rhs)`](../expression_methods/trait.ExpressionMethods.html#method.le)
pub type LtEq<Lhs, Rhs> = super::operators::LtEq<Lhs, AsExpr<Rhs, Lhs>>;

/// The return type of
/// [`lhs.between(lower, upper)`](../expression_methods/trait.ExpressionMethods.html#method.between)
pub type Between<Lhs, Lower, Upper> =
    super::operators::Between<Lhs, super::operators::And<AsExpr<Lower, Lhs>, AsExpr<Upper, Lhs>>>;

/// The return type of
/// [`lhs.not_between(lower, upper)`](../expression_methods/trait.ExpressionMethods.html#method.not_between)
pub type NotBetween<Lhs, Lower, Upper> = super::operators::NotBetween<
    Lhs,
    super::operators::And<AsExpr<Lower, Lhs>, AsExpr<Upper, Lhs>>,
>;

/// The return type of
/// [`expr.desc()`](../expression_methods/trait.ExpressionMethods.html#method.desc)
pub type Desc<Expr> = super::operators::Desc<Expr>;

/// The return type of
/// [`expr.asc()`](../expression_methods/trait.ExpressionMethods.html#method.asc)
pub type Asc<Expr> = super::operators::Asc<Expr>;

/// The return type of
/// [`expr.nullable()`](../expression_methods/trait.NullableExpressionMethods.html#method.nullable)
pub type Nullable<Expr> = super::nullable::Nullable<Expr>;

/// The return type of
/// [`lhs.and(rhs)`](../expression_methods/trait.BoolExpressionMethods.html#method.and)
pub type And<Lhs, Rhs> = super::operators::And<Lhs, AsExprOf<Rhs, sql_types::Bool>>;

/// The return type of
/// [`lhs.or(rhs)`](../expression_methods/trait.BoolExpressionMethods.html#method.or)
pub type Or<Lhs, Rhs> = Grouped<super::operators::Or<Lhs, AsExprOf<Rhs, sql_types::Bool>>>;

/// The return type of
/// [`lhs.escape('x')`](../expression_methods/trait.EscapeExpressionMethods.html#method.escape)
pub type Escape<Lhs> = super::operators::Escape<Lhs, AsExprOf<String, sql_types::VarChar>>;

/// The return type of
/// [`lhs.like(rhs)`](../expression_methods/trait.TextExpressionMethods.html#method.like)
pub type Like<Lhs, Rhs> = super::operators::Like<Lhs, AsExprOf<Rhs, sql_types::VarChar>>;

/// The return type of
/// [`lhs.not_like(rhs)`](../expression_methods/trait.TextExpressionMethods.html#method.not_like)
pub type NotLike<Lhs, Rhs> = super::operators::NotLike<Lhs, AsExprOf<Rhs, sql_types::VarChar>>;

#[doc(inline)]
pub use super::functions::helper_types::*;
