#[doc(hidden)]
pub mod array_comparison;
pub mod expression_methods;
pub mod extensions;
#[doc(hidden)]
pub mod operators;
#[doc(hidden)]
pub mod helper_types;

mod date_and_time;

/// PostgreSQL specific expression DSL methods. This module will be glob
/// imported by [`diesel::dsl`](../../dsl/index.html) when
/// compiled with the `feature = "postgres"` flag.
pub mod dsl {
    #[doc(inline)]
    pub use super::array_comparison::{all, any};

    pub use super::extensions::*;
}
