#![allow(non_camel_case_types)]

use crate::dsl::SqlTypeOf;
use crate::expression::grouped::Grouped;
use crate::expression::operators;

/// The return type of [`not(expr)`](super::dsl::not())
pub type not<Expr> = operators::Not<Grouped<Expr>>;

/// The return type of [`max(expr)`](super::dsl::max())
pub type max<Expr> = super::aggregate_ordering::max::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`min(expr)`](super::dsl::min())
pub type min<Expr> = super::aggregate_ordering::min::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`sum(expr)`](super::dsl::sum())
pub type sum<Expr> = super::aggregate_folding::sum::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`avg(expr)`](super::dsl::avg())
pub type avg<Expr> = super::aggregate_folding::avg::HelperType<SqlTypeOf<Expr>, Expr>;
