#![allow(non_camel_case_types)]

use crate::dsl::SqlTypeOf;
use crate::expression::grouped::Grouped;
use crate::expression::operators;

/// The return type of [`not(expr)`](crate::dsl::not())
pub type Not<Expr> = operators::Not<Grouped<Expr>>;

#[doc(hidden)]
// cannot put deprecated on this because rustc then
// also reports the function as deprecated
#[cfg(any(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type not<Expr> = Not<Expr>;

/// The return type of [`max(expr)`](crate::dsl::max())
pub type Max<Expr> = super::aggregate_ordering::max::HelperType<SqlTypeOf<Expr>, Expr>;

#[doc(hidden)]
// cannot put deprecated on this because rustc then
// also reports the function as deprecated
#[cfg(any(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type max<Expr> = Max<Expr>;

/// The return type of [`min(expr)`](crate::dsl::min())
pub type Min<Expr> = super::aggregate_ordering::min::HelperType<SqlTypeOf<Expr>, Expr>;

#[doc(hidden)]
// cannot put deprecated on this because rustc then
// also reports the function as deprecated
#[cfg(any(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type min<Expr> = Min<Expr>;

/// The return type of [`sum(expr)`](crate::dsl::sum())
pub type Sum<Expr> = super::aggregate_folding::sum::HelperType<SqlTypeOf<Expr>, Expr>;

#[doc(hidden)]
// cannot put deprecated on this because rustc then
// also reports the function as deprecated
#[cfg(any(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type sum<Expr> = Sum<Expr>;

/// The return type of [`avg(expr)`](crate::dsl::avg())
pub type Avg<Expr> = super::aggregate_folding::avg::HelperType<SqlTypeOf<Expr>, Expr>;

#[doc(hidden)]
// cannot put deprecated on this because rustc then
// also reports the function as deprecated
#[cfg(any(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type avg<Expr> = Avg<Expr>;

/// The return type of [`exists(expr)`](crate::dsl::exists())
pub type Exists<Expr> = crate::expression::exists::Exists<Expr>;

#[doc(hidden)]
// cannot put deprecated on this because rustc then
// also reports the function as deprecated
#[cfg(any(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type exists<Expr> = Exists<Expr>;
