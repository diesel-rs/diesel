//! Provides types and functions related to working with SQLite
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! SQLite, you may need to work with this module directly.

pub(crate) mod backend;
mod connection;
pub(crate) mod expression;

pub mod query_builder;

mod types;

pub use self::backend::{Sqlite, SqliteType};
pub use self::connection::SerializedDatabase;
pub use self::connection::SqliteBindValue;
pub use self::connection::SqliteConnection;
pub use self::connection::SqliteValue;
pub use self::query_builder::SqliteQueryBuilder;

/// Trait for the implementation of a SQLite aggregate function
///
/// This trait is to be used in conjunction with the `define_sql_function!`
/// macro for defining a custom SQLite aggregate function. See
/// the documentation [there](super::prelude::define_sql_function!) for details.
pub trait SqliteAggregateFunction<Args>: Default {
    /// The result type of the SQLite aggregate function
    type Output;

    /// The `step()` method is called once for every record of the query.
    ///
    /// This is called through a C FFI, as such panics do not propagate to the caller. Panics are
    /// caught and cause a return with an error value. The implementation must still ensure that
    /// state remains in a valid state (refer to [`std::panic::UnwindSafe`] for a bit more detail).
    fn step(&mut self, args: Args);

    /// After the last row has been processed, the `finalize()` method is
    /// called to compute the result of the aggregate function. If no rows
    /// were processed `aggregator` will be `None` and `finalize()` can be
    /// used to specify a default result.
    ///
    /// This is called through a C FFI, as such panics do not propagate to the caller. Panics are
    /// caught and cause a return with an error value.
    fn finalize(aggregator: Option<Self>) -> Self::Output;
}

/// SQLite specific sql types
pub mod sql_types {
    #[doc(inline)]
    pub use super::types::Timestamptz;
}
