//! Sqlite related query builder extensions.
//!
//! Everything in this module is re-exported from database agnostic locations.
//! You should rely on the re-exports rather than this module directly. It is
//! kept separate purely for documentation purposes.

pub(crate) mod expression_methods;
pub mod functions;
pub(crate) mod helper_types;
mod operators;

mod return_type_helpers {
    #[allow(unused_imports)]
    #[doc(inline)]
    pub use super::functions::return_type_helpers_reexported::*;
}

/// SQLite specific expression DSL methods.
///
/// This module will be glob imported by
/// [`diesel::dsl`](crate::dsl) when compiled with the `feature =
/// "sqlite"` flag.
pub mod dsl {
    #[doc(inline)]
    pub use super::functions::*;
}
