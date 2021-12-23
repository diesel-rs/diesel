//! PostgreSQL related query builder extensions
//!
//! Everything in this module is re-exported from database agnostic locations.
//! You should rely on the re-exports rather than this module directly. It is
//! kept separate purely for documentation purposes.

pub(crate) mod array;
#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub mod array_comparison;
pub(crate) mod expression_methods;
pub mod extensions;
pub mod functions;
#[doc(hidden)]
pub mod helper_types;
#[doc(hidden)]
pub mod operators;

mod date_and_time;

/// PostgreSQL specific expression DSL methods.
///
/// This module will be glob imported by
/// [`diesel::dsl`](crate::dsl) when compiled with the `feature =
/// "postgres"` flag.
pub mod dsl {
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    #[doc(inline)]
    #[allow(deprecated)]
    pub use super::array_comparison::{all, any};

    #[doc(inline)]
    pub use super::array::array;

    pub use super::extensions::*;

    #[cfg(not(feature = "sqlite"))]
    pub use super::functions::*;
}
