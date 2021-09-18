#![allow(non_camel_case_types)]

use crate::dsl::SqlTypeOf;
use crate::expression::grouped::Grouped;
use crate::expression::operators;

/// The return type of [`not(expr)`](crate::dsl::not())
pub type not<Expr> = operators::Not<Grouped<Expr>>;

/// The return type of [`max(expr)`](crate::dsl::max())
pub type max<Expr> = super::aggregate_ordering::max::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`min(expr)`](crate::dsl::min())
pub type min<Expr> = super::aggregate_ordering::min::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`sum(expr)`](crate::dsl::sum())
pub type sum<Expr> = super::aggregate_folding::sum::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`avg(expr)`](crate::dsl::avg())
pub type avg<Expr> = super::aggregate_folding::avg::HelperType<SqlTypeOf<Expr>, Expr>;

/// The return type of [`exists(expr)`](crate::dsl::exists())
pub type exists<Expr> = crate::expression::exists::Exists<Expr>;
