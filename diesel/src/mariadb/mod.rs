//! Provides types and functions related to working with MariaDB
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MariaDB, you may need to work with this module directly.
pub(crate) mod backend;

//pub(crate) mod query_builder;

pub use self::backend::{Mariadb, MariadbType};

#[cfg(feature = "mariadb")]
mod connection;

#[cfg(feature = "mariadb")]
pub use self::connection::MariadbConnection;

/// The Mariadb query builder
pub type MariadbQueryBuilder = crate::mysql_like::query_builder::MysqlLikeQueryBuilder<Mariadb>;

/// Raw mariadb value as received from the database
pub type MariadbValue<'a> = crate::mysql_like::MysqlValue<'a>;
pub use crate::mysql_like::NumericRepresentation;

pub use crate::mysql_like::data_types;

pub use crate::mysql_like::sql_types;
