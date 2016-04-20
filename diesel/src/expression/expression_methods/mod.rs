//! Adds various methods to construct new expressions. These traits are exported
//! by default, and implemented automatically.
//!
//! You can rely on the methods provided by this trait existing on any
//! `Expression` of the appropriate type. You should not rely on the specific
//! traits existing, their names, or their organization.
pub mod bool_expression_methods;
pub mod escape_expression_methods;
pub mod global_expression_methods;
pub mod text_expression_methods;

pub use self::bool_expression_methods::BoolExpressionMethods;
pub use self::escape_expression_methods::EscapeExpressionMethods;
pub use self::global_expression_methods::ExpressionMethods;
pub use self::text_expression_methods::TextExpressionMethods;

#[cfg(feature = "postgres")]
pub use pg::expression::expression_methods::*;
