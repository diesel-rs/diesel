//! Provides types and functions related to working with SQLite
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! SQLite, you may need to work with this module directly.

mod auto_extension;
pub(crate) mod backend;
pub(crate) mod connection;
pub mod expression;
mod function_behavior;

pub mod query_builder;

mod types;

pub use self::auto_extension::cancel_auto_extension;
pub use self::auto_extension::register_auto_extension;
pub use self::auto_extension::reset_auto_extension;
pub use self::backend::{Sqlite, SqliteType};
pub use self::connection::AutoVacuumMode;
pub use self::connection::BusyDecision;
pub use self::connection::CommitDecision;
pub use self::connection::ProgressDecision;
pub use self::connection::SerializedDatabase;
pub use self::connection::SqliteBindValue;
pub use self::connection::SqliteConnection;
pub use self::connection::SqliteLimit;
pub use self::connection::SqliteTraceEvent;
pub use self::connection::SqliteTraceFlags;
pub use self::connection::SqliteValue;
pub use self::connection::WalCheckpointMode;
pub use self::connection::WalCheckpointOutcome;
pub use self::connection::authorizer;
pub use self::connection::sqlite_blob::SqliteReadOnlyBlob;
pub use self::connection::{AuthorizerContext, AuthorizerDecision};
pub use self::connection::{CollationNeededContext, SqliteTextRep};
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::connection::{
    OwnedSqliteBindValue, SqliteBindCollector, SqliteBindCollectorData, SqliteBindValueRef,
};
pub use self::connection::{
    SqliteChangeEvent, SqliteChangeOp, SqliteChangeOps, SqliteUpdateRouter,
};
#[cfg(feature = "__sqlite-shared")]
pub use self::function_behavior::SqliteFunctionBehavior;
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

/// Trait for the implementation of a SQLite aggregate window function
///
/// Implementing this trait in addition to [`SqliteAggregateFunction`] lets a
/// custom aggregate also run as a window function inside an `OVER` clause
/// (requires SQLite 3.25.0 or newer). See the
/// [`define_sql_function!`](super::prelude::define_sql_function!)
/// documentation for details.
///
/// SQLite substitutes [`finalize`](SqliteAggregateFunction::finalize) for
/// [`value`](Self::value) on frames it cannot compute incrementally and at
/// partition ends, so both must return the same result for the same state.
///
/// Panics in `value` and `inverse` are caught at the FFI boundary and
/// reported as query errors. The implementation must still keep the state
/// valid (refer to [`std::panic::UnwindSafe`] for a bit more detail).
#[diagnostic::on_unimplemented(
    note = "implement `SqliteWindowFunction` in addition to `SqliteAggregateFunction` to register `{Self}` for a window function"
)]
pub trait SqliteWindowFunction<Args>: SqliteAggregateFunction<Args> {
    /// Returns the current value of the aggregate without consuming the
    /// state. `aggregator` is `None` when the window frame contains no rows.
    fn value(aggregator: Option<&Self>) -> Self::Output;

    /// Removes the oldest row from the window, given the same arguments
    /// [`step`](SqliteAggregateFunction::step) received when adding it.
    fn inverse(&mut self, args: Args);
}

/// SQLite specific sql types
pub mod sql_types {
    #[doc(inline)]
    pub use super::types::Timestamptz;

    #[cfg(feature = "__sqlite-shared")]
    #[doc(inline)]
    pub use super::types::JsonValidFlags;
}

#[cfg(feature = "__sqlite-shared")]
pub use self::types::JsonValidFlag;
