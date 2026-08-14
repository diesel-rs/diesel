//! Provides types and functions related to working with MySQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MySQL, you may need to work with this module directly.

pub(crate) mod backend;
#[cfg(feature = "mysql")]
mod connection;

use crate::mysql_like::query_builder::MysqlLikeQueryBuilder;

#[doc(inline)]
pub use self::backend::Mysql;
#[cfg(feature = "mysql")]
#[doc(inline)]
pub use self::connection::MysqlConnection;
#[doc(inline)]
pub use super::mysql_like::sql_types;
#[doc(inline)]
pub use super::mysql_like::{MysqlType, MysqlValue, NumericRepresentation};
#[doc(inline)]
pub use crate::mysql_like::data_types;

/// The MySQL query builder
pub type MysqlQueryBuilder = MysqlLikeQueryBuilder<Mysql>;
