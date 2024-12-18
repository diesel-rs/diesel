use crate::dsl::{AsExpr, SqlTypeOf};
use crate::expression::grouped::Grouped;

/// The return type of `lhs.is(rhs)`.
pub type Is<Lhs, Rhs> = Grouped<super::operators::Is<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.is_not(rhs)`.
pub type IsNot<Lhs, Rhs> = Grouped<super::operators::IsNot<Lhs, AsExpr<Rhs, Lhs>>>;

/// Return type of [`json(json)`](super::functions::json())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json<E> = super::functions::json<SqlTypeOf<E>, E>;

/// Return type of [`jsonb(json)`](super::functions::jsonb())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb<E> = super::functions::jsonb<SqlTypeOf<E>, E>;

/// Return type of [`json_pretty(json)`](super::functions::json_pretty())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_pretty<E> = super::functions::json_pretty<SqlTypeOf<E>, E>;
