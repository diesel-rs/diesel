//! Provides types and functions related to working with MySQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MySQL, you may need to work with this module directly.

pub(crate) mod backend;
#[cfg(feature = "mysql")]
mod connection;

use crate::mysql_like::query_builder::MysqlLikeQueryBuilder;

pub use self::backend::Mysql;
pub use super::mysql_like::MysqlType;
#[cfg(feature = "mysql")]
pub use self::connection::MysqlConnection;
pub use super::mysql_like::{MysqlValue, NumericRepresentation};

/// The MySQL query builder
pub type MysqlQueryBuilder = MysqlLikeQueryBuilder<Mysql>;
