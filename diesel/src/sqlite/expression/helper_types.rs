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

/// Return type of [`json_pretty(json, indent)`](super::functions::json_pretty_with_indentation())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_pretty_with_indentation<J, I> =
    super::functions::json_pretty_with_indentation<SqlTypeOf<J>, J, I>;

/// Return type of [`json_type(json)`](super::functions::json_type())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_type<J> = super::functions::json_type<SqlTypeOf<J>, J>;

/// Return type of [`json_type_with_path(json, path)`](super::functions::json_type_with_path())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_type_with_path<J, P> = super::functions::json_type_with_path<SqlTypeOf<J>, J, P>;
