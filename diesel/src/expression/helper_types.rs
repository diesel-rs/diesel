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
/// [`lhs.eq(rhs)`](crate::expression_methods::ExpressionMethods::eq())
pub type Eq<Lhs, Rhs> = Grouped<super::operators::Eq<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`lhs.ne(rhs)`](crate::expression_methods::ExpressionMethods::ne())
pub type NotEq<Lhs, Rhs> = Grouped<super::operators::NotEq<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`lhs.eq_any(rhs)`](crate::expression_methods::ExpressionMethods::eq_any())
pub type EqAny<Lhs, Rhs> = Grouped<In<Lhs, <Rhs as AsInExpression<SqlTypeOf<Lhs>>>::InExpression>>;

/// The return type of
/// [`lhs.ne_all(rhs)`](crate::expression_methods::ExpressionMethods::ne_all())
pub type NeAny<Lhs, Rhs> =
    Grouped<NotIn<Lhs, <Rhs as AsInExpression<SqlTypeOf<Lhs>>>::InExpression>>;

/// The return type of
/// [`expr.is_null()`](crate::expression_methods::ExpressionMethods::is_null())
pub type IsNull<Expr> = Grouped<super::operators::IsNull<Expr>>;

/// The return type of
/// [`expr.is_not_null()`](crate::expression_methods::ExpressionMethods::is_not_null())
pub type IsNotNull<Expr> = Grouped<super::operators::IsNotNull<Expr>>;

/// The return type of
/// [`lhs.gt(rhs)`](crate::expression_methods::ExpressionMethods::gt())
pub type Gt<Lhs, Rhs> = Grouped<super::operators::Gt<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`lhs.ge(rhs)`](crate::expression_methods::ExpressionMethods::ge())
pub type GtEq<Lhs, Rhs> = Grouped<super::operators::GtEq<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`lhs.lt(rhs)`](crate::expression_methods::ExpressionMethods::lt())
pub type Lt<Lhs, Rhs> = Grouped<super::operators::Lt<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`lhs.le(rhs)`](crate::expression_methods::ExpressionMethods::le())
pub type LtEq<Lhs, Rhs> = Grouped<super::operators::LtEq<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`lhs.between(lower, upper)`](crate::expression_methods::ExpressionMethods::between())
pub type Between<Lhs, Lower, Upper> = Grouped<
    super::operators::Between<Lhs, super::operators::And<AsExpr<Lower, Lhs>, AsExpr<Upper, Lhs>>>,
>;

/// The return type of
/// [`lhs.not_between(lower, upper)`](crate::expression_methods::ExpressionMethods::not_between())
pub type NotBetween<Lhs, Lower, Upper> = Grouped<
    super::operators::NotBetween<
        Lhs,
        super::operators::And<AsExpr<Lower, Lhs>, AsExpr<Upper, Lhs>>,
    >,
>;

/// The return type of
/// [`lhs.concat(rhs)`](crate::expression_methods::TextExpressionMethods::concat())
pub type Concat<Lhs, Rhs> = Grouped<super::operators::Concat<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of
/// [`expr.desc()`](crate::expression_methods::ExpressionMethods::desc())
pub type Desc<Expr> = super::operators::Desc<Expr>;

/// The return type of
/// [`expr.asc()`](crate::expression_methods::ExpressionMethods::asc())
pub type Asc<Expr> = super::operators::Asc<Expr>;

/// The return type of
/// [`expr.nullable()`](crate::expression_methods::NullableExpressionMethods::nullable())
pub type Nullable<Expr> = super::nullable::Nullable<Expr>;

/// The return type of
/// [`lhs.and(rhs)`](crate::expression_methods::BoolExpressionMethods::and())
pub type And<Lhs, Rhs, ST = sql_types::Bool> =
    Grouped<super::operators::And<Lhs, AsExprOf<Rhs, ST>>>;

/// The return type of
/// [`lhs.or(rhs)`](crate::expression_methods::BoolExpressionMethods::or())
pub type Or<Lhs, Rhs, ST = sql_types::Bool> = Grouped<super::operators::Or<Lhs, AsExprOf<Rhs, ST>>>;

/// The return type of
/// [`lhs.escape('x')`](crate::expression_methods::EscapeExpressionMethods::escape())
pub type Escape<Lhs> = Grouped<
    super::operators::Escape<
        <Lhs as crate::expression_methods::EscapeExpressionMethods>::TextExpression,
        AsExprOf<String, sql_types::VarChar>,
    >,
>;

/// The return type of
/// [`lhs.like(rhs)`](crate::expression_methods::TextExpressionMethods::like())
pub type Like<Lhs, Rhs> = Grouped<super::operators::Like<Lhs, AsExprOf<Rhs, SqlTypeOf<Lhs>>>>;

/// The return type of
/// [`lhs.not_like(rhs)`](crate::expression_methods::TextExpressionMethods::not_like())
pub type NotLike<Lhs, Rhs> = Grouped<super::operators::NotLike<Lhs, AsExprOf<Rhs, SqlTypeOf<Lhs>>>>;

#[doc(inline)]
pub use super::functions::helper_types::*;

#[doc(inline)]
#[cfg(feature = "postgres")]
pub use crate::pg::expression::helper_types::*;

#[doc(inline)]
#[cfg(feature = "sqlite")]
pub use crate::sqlite::expression::helper_types::*;
