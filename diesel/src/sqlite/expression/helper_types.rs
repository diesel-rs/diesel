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

/// Return type of [`json_group_array(value)`](super::functions::json_group_array())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_group_array<E> = super::functions::json_group_array<SqlTypeOf<E>, E>;

/// Return type of [`jsonb_group_array(value)`](super::functions::jsonb_group_array())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_group_array<E> = super::functions::jsonb_group_array<SqlTypeOf<E>, E>;

/// Return type of [`json_group_object(names, values)`](super::functions::json_group_object())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_group_object<N, V> =
    super::functions::json_group_object<SqlTypeOf<N>, SqlTypeOf<V>, N, V>;

/// Return type of [`jsonb_group_object(names, values)`](super::functions::jsonb_group_object())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_group_object<N, V> =
    super::functions::jsonb_group_object<SqlTypeOf<N>, SqlTypeOf<V>, N, V>;

/// Return type of [`json_array_0()`](super::functions::json_array_0())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_array_0 = super::functions::json_array_0;

/// Return type of [`json_array_1(value_1)`](super::functions::json_array_1())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_array_1<V1> = super::functions::json_array_1<SqlTypeOf<V1>, V1>;

/// Return type of [`json_array_2(value_1, value_2)`](super::functions::json_array_2())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_array_2<V1, V2> =
    super::functions::json_array_2<SqlTypeOf<V1>, SqlTypeOf<V2>, V1, V2>;

/// Return type of [`jsonb_array_0()`](super::functions::jsonb_array_0())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_array_0 = super::functions::jsonb_array_0;

/// Return type of [`jsonb_array_1(value_1)`](super::functions::jsonb_array_1())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_array_1<V1> = super::functions::jsonb_array_1<SqlTypeOf<V1>, V1>;

/// Return type of [`jsonb_array_2(value_1, value_2)`](super::functions::jsonb_array_2())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_array_2<V1, V2> =
    super::functions::jsonb_array_2<SqlTypeOf<V1>, SqlTypeOf<V2>, V1, V2>;

/// Return type of [`json_remove_0(json)`](super::functions::json_remove_0())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_remove_0<J> = super::functions::json_remove_0<SqlTypeOf<J>, J>;

/// Return type of [`json_remove_1(json, path_1)`](super::functions::json_remove_1())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_remove_1<J, P1> = super::functions::json_remove_1<SqlTypeOf<J>, J, P1>;

/// Return type of [`json_remove_2(json, path_1, path_2)`](super::functions::json_remove_2())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type json_remove_2<J, P1, P2> = super::functions::json_remove_2<SqlTypeOf<J>, J, P1, P2>;

/// Return type of [`jsonb_remove_0(json)`](super::functions::jsonb_remove_0())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_remove_0<J> = super::functions::jsonb_remove_0<SqlTypeOf<J>, J>;

/// Return type of [`jsonb_remove_1(json, path_1)`](super::functions::jsonb_remove_1())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_remove_1<J, P1> = super::functions::jsonb_remove_1<SqlTypeOf<J>, J, P1>;

/// Return type of [`jsonb_remove_2(json, path_1, path_2)`](super::functions::jsonb_remove_2())
#[allow(non_camel_case_types)]
#[cfg(feature = "sqlite")]
pub type jsonb_remove_2<J, P1, P2> = super::functions::jsonb_remove_2<SqlTypeOf<J>, J, P1, P2>;
