//! PostgreSQL related query builder extensions
//!
//! Everything in this module is re-exported from database agnostic locations.
//! You should rely on the re-exports rather than this module directly. It is
//! kept separate purely for documentation purposes.

#[doc(hidden)]
pub mod array_comparison;
pub(crate) mod expression_methods;
#[doc(hidden)]
pub mod array;
pub mod extensions;
#[doc(hidden)]
pub mod operators;
#[doc(hidden)]
pub mod helper_types;

mod date_and_time;

/// PostgreSQL specific expression DSL methods. This module will be glob
/// imported by [`diesel::dsl`](../../../dsl/index.html) when
/// compiled with the `feature = "postgres"` flag.
pub mod dsl {
    #[doc(inline)]
    pub use super::array_comparison::{all, any};

    #[doc(inline)]
    pub use super::array::array;

    pub use super::extensions::*;
}
