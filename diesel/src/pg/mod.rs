pub mod expression;

mod backend;
mod metadata_lookup;
mod query_builder;
mod connection;
pub mod types;
pub mod upsert;

pub use self::backend::{Pg, PgTypeMetadata};
pub use self::connection::PgConnection;
pub use self::metadata_lookup::PgMetadataLookup;
pub use self::query_builder::PgQueryBuilder;
pub use self::query_builder::DistinctOnClause;

pub mod data_types {
    #[doc(inline)]
    pub use super::types::date_and_time::{PgDate, PgInterval, PgTime, PgTimestamp};
    #[doc(inline)]
    pub use super::types::floats::PgNumeric;
    #[doc(inline)]
    pub use super::types::money::PgMoney;
    pub use super::types::money::PgMoney as Cents;
}
