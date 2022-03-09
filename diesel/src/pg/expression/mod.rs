//! PostgreSQL related query builder extensions
//!
//! Everything in this module is re-exported from database agnostic locations.
//! You should rely on the re-exports rather than this module directly. It is
//! kept separate purely for documentation purposes.

pub(crate) mod array;
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub(crate) mod array_comparison;
pub(crate) mod expression_methods;
pub mod extensions;
pub mod functions;
pub(crate) mod helper_types;
pub(crate) mod operators;

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

    #[doc(inline)]
    pub use super::extensions::*;

    #[doc(inline)]
    pub use super::functions::*;
}
