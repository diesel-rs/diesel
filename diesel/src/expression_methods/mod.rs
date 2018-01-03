//! Adds various methods to construct new expressions. These traits are exported
//! by default, and implemented automatically.
//!
//! You can rely on the methods provided by this trait existing on any
//! `Expression` of the appropriate type. You should not rely on the specific
//! traits existing, their names, or their organization.
mod bool_expression_methods;
mod escape_expression_methods;
mod global_expression_methods;
mod text_expression_methods;
mod eq_all;

#[doc(inline)]
pub use self::bool_expression_methods::BoolExpressionMethods;
#[doc(inline)]
pub use self::escape_expression_methods::EscapeExpressionMethods;
#[doc(inline)]
pub use self::global_expression_methods::{ExpressionMethods, NullableExpressionMethods};
#[doc(inline)]
pub use self::text_expression_methods::TextExpressionMethods;
#[doc(hidden)]
pub use self::eq_all::EqAll;

#[cfg(feature = "postgres")]
#[doc(inline)]
pub use pg::expression::expression_methods::*;
