#![allow(non_camel_case_types)]

use crate::dsl::SqlTypeOf;
use crate::expression::grouped::Grouped;
use crate::expression::operators;

/// The return type of [`not(expr)`](crate::dsl::not())
pub type not<Expr> = operators::Not<Grouped<Expr>>;

/// The return type of [`max(expr)`](crate::dsl::max())
pub type max<Expr> = super::aggregate_ordering::max<SqlTypeOf<Expr>, Expr>;

/// The return type of [`min(expr)`](crate::dsl::min())
pub type min<Expr> = super::aggregate_ordering::min<SqlTypeOf<Expr>, Expr>;

/// The return type of [`sum(expr)`](crate::dsl::sum())
pub type sum<Expr> = super::aggregate_folding::sum<SqlTypeOf<Expr>, Expr>;

/// The return type of [`avg(expr)`](crate::dsl::avg())
pub type avg<Expr> = super::aggregate_folding::avg<SqlTypeOf<Expr>, Expr>;

/// The return type of [`exists(expr)`](crate::dsl::exists())
pub type exists<Expr> = crate::expression::exists::Exists<Expr>;

/// The return type of [`lag(expr)`](crate::dsl::lag())
pub type lag<Expr> = super::window_functions::lag<SqlTypeOf<Expr>, Expr>;
/// The return type of [`lag_with_offset(expr, offset)`](crate::dsl::lag_with_offset())
pub type lag_with_offset<V, O> = super::window_functions::lag_with_offset<SqlTypeOf<V>, V, O>;
/// The return type of [`lag_with_offset_and_default(expr, offset)`](crate::dsl::lag_with_offset_and_default())
pub type lag_with_offset_and_default<V, O, D> =
    super::window_functions::lag_with_offset_and_default<SqlTypeOf<V>, SqlTypeOf<D>, V, O, D>;
/// The return type of [`lead(expr)`](crate::dsl::lead())
pub type lead<Expr> = super::window_functions::lead<SqlTypeOf<Expr>, Expr>;
/// The return type of [`lead_with_offset(expr, offset)`](crate::dsl::lead_with_offset())
pub type lead_with_offset<V, O> = super::window_functions::lead_with_offset<SqlTypeOf<V>, V, O>;
/// The return type of [`lead_with_offset_and_default(expr, offset)`](crate::dsl::lead_with_offset_and_default())
pub type lead_with_offset_and_default<V, O, D> =
    super::window_functions::lead_with_offset_and_default<SqlTypeOf<V>, SqlTypeOf<D>, V, O, D>;
/// The return type of [`first_value(expr)`](crate::dsl::first_value())
pub type first_value<Expr> = super::window_functions::first_value<SqlTypeOf<Expr>, Expr>;
/// The return type of [`last_value(expr)`](crate::dsl::last_value())
pub type last_value<Expr> = super::window_functions::last_value<SqlTypeOf<Expr>, Expr>;
/// The return type of [`nth_value(expr, n)`](crate::dsl::nth_value())
pub type nth_value<V, N> = super::window_functions::nth_value<SqlTypeOf<V>, V, N>;

#[doc(inline)]
pub use super::aggregate_expressions::dsl::*;
