//! Provides types and functions related to working with PostgreSQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! PostgreSQL, you may need to work with this module directly.

pub mod expression;
pub mod types;

mod backend;
mod connection;
mod metadata_lookup;
pub(crate) mod query_builder;
pub(crate) mod serialize;
mod transaction;
mod value;

pub use self::backend::{Pg, PgTypeMetadata};
pub use self::connection::PgConnection;
#[doc(hidden)]
pub use self::metadata_lookup::{GetPgMetadataCache, PgMetadataCache, PgMetadataLookup};
pub use self::query_builder::DistinctOnClause;
pub use self::query_builder::PgQueryBuilder;
pub use self::transaction::TransactionBuilder;
pub use self::value::PgValue;
#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(since = "2.0.0", note = "Use `diesel::upsert` instead")]
pub use crate::upsert;

/// Data structures for PG types which have no corresponding Rust type
///
/// Most of these types are used to implement `ToSql` and `FromSql` for higher
/// level types.
pub mod data_types {
    #[doc(inline)]
    pub use super::types::date_and_time::{PgDate, PgInterval, PgTime, PgTimestamp};
    #[doc(inline)]
    pub use super::types::floats::PgNumeric;
    #[doc(inline)]
    pub use super::types::money::PgMoney;
    pub use super::types::money::PgMoney as Cents;
}
