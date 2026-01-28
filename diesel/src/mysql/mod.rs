//! Provides types and functions related to working with MySQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MySQL, you may need to work with this module directly.

pub(crate) mod backend;
#[cfg(feature = "mysql")]
mod connection;
mod value;

pub(crate) mod query_builder;
mod types;

pub use self::backend::{Mysql, MysqlType};
#[cfg(feature = "mysql")]
pub use self::connection::MysqlConnection;
pub use self::query_builder::MysqlQueryBuilder;
pub use self::value::{MysqlValue, NumericRepresentation};

/// Data structures for MySQL types which have no corresponding Rust type
///
/// Most of these types are used to implement `ToSql` and `FromSql` for higher
/// level types.
pub mod data_types {
    #[doc(inline)]
    pub use super::types::date_and_time::{MysqlTime, MysqlTimestampType};
}

/// MySQL specific sql types
pub mod sql_types {
    #[doc(inline)]
    pub use super::types::{Datetime, Unsigned};
}
