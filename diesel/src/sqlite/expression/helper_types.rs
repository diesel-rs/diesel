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

/// Return type of [`json_array_length(json)`](super::functions::json_array_length())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_array_length<J> = super::functions::json_array_length<SqlTypeOf<J>, J>;

/// Return type of [`json_array_length(json, path)`](super::functions::json_array_length_with_path())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_array_length_with_path<J, P> =
    super::functions::json_array_length_with_path<SqlTypeOf<J>, J, P>;

/// Return type of [`json_error_position(json)`](super::functions::json_error_position())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_error_position<X> = super::functions::json_error_position<SqlTypeOf<X>, X>;

/// Return type of [`json_pretty(json)`](super::functions::json_pretty())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_pretty<E> = super::functions::json_pretty<SqlTypeOf<E>, E>;

/// Return type of [`json_pretty(json, indent)`](super::functions::json_pretty_with_indentation())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_pretty_with_indentation<J, I> =
    super::functions::json_pretty_with_indentation<SqlTypeOf<J>, J, I>;

/// Return type of [`json_valid(json)`](super::functions::json_valid())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_valid<E> = super::functions::json_valid<SqlTypeOf<E>, E>;

/// Return type of [`json_type(json)`](super::functions::json_type())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_type<E> = super::functions::json_type<SqlTypeOf<E>, E>;

/// Return type of [`json_type(json, path)`](super::functions::json_type_with_path())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_type_with_path<J, P> = super::functions::json_type_with_path<SqlTypeOf<J>, J, P>;

/// Return type of [`json_quote(value)`](super::functions::json_quote())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_quote<J> = super::functions::json_quote<SqlTypeOf<J>, J>;

/// Return type of [`json_patch(json, json)`](super::functions::json_patch())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_patch<J, P> = super::functions::json_patch<SqlTypeOf<J>, J, P>;

/// Return type of [`jsonb_patch(json, json)`](super::functions::jsonb_patch())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_patch<J, P> = super::functions::jsonb_patch<SqlTypeOf<J>, J, P>;
