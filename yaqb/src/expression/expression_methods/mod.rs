pub mod global_expression_methods;
pub mod bool_expression_methods;
pub mod text_expression_methods;

pub use self::global_expression_methods::ExpressionMethods;
pub use self::bool_expression_methods::BoolExpressionMethods;
pub use self::text_expression_methods::{TextExpressionMethods, VarCharExpressionMethods};
