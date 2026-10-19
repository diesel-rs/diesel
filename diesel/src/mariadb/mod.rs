//! Provides types and functions related to working with MariaDB
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MariaDB, you may need to work with this module directly.
pub(crate) mod backend;
#[cfg(feature = "mariadb")]
mod connection;

mod query_fragment_impls;

#[doc(inline)]
pub use self::backend::{Mariadb, MariadbType};

#[doc(inline)]
#[cfg(feature = "mariadb")]
pub use self::connection::MariadbConnection;

/// The Mariadb query builder
pub type MariadbQueryBuilder = crate::mysql_like::query_builder::MysqlLikeQueryBuilder<Mariadb>;

/// Raw mariadb value as received from the database
pub type MariadbValue<'a> = crate::mysql_like::MysqlValue<'a>;
#[doc(inline)]
pub use crate::mysql_like::NumericRepresentation;
#[doc(inline)]
pub use crate::mysql_like::data_types;
#[doc(inline)]
pub use crate::mysql_like::sql_types;

pub mod returning;
