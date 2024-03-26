//! Provides types and functions related to working with PostgreSQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! PostgreSQL, you may need to work with this module directly.

pub mod expression;
mod types;

pub(crate) mod backend;
#[cfg(feature = "postgres")]
pub(crate) mod connection;
mod metadata_lookup;
pub(crate) mod query_builder;
pub(crate) mod serialize;
mod transaction;
mod value;

pub use self::backend::{Pg, PgTypeMetadata};
#[cfg(feature = "postgres")]
pub use self::connection::{PgConnection, PgRowByRowLoadingMode};
#[doc(inline)]
pub use self::metadata_lookup::PgMetadataLookup;
#[doc(inline)]
pub use self::query_builder::DistinctOnClause;
#[doc(inline)]
pub use self::query_builder::OrderDecorator;
#[doc(inline)]
pub use self::query_builder::PgQueryBuilder;
#[doc(inline)]
pub use self::query_builder::{CopyFormat, CopyFromQuery, CopyHeader, CopyTarget, CopyToQuery};
#[doc(inline)]
pub use self::transaction::TransactionBuilder;
#[doc(inline)]
pub use self::value::PgValue;

#[doc(inline)]
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::backend::FailedToLookupTypeError;
#[doc(inline)]
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::metadata_lookup::{GetPgMetadataCache, PgMetadataCache, PgMetadataCacheKey};
#[doc(inline)]
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::value::TypeOidLookup;

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

#[doc(inline)]
pub use self::types::sql_types;
