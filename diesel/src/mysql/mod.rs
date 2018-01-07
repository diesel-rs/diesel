//! Provides types and functions related to working with MySQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MySQL, you may need to work with this module directly.

mod backend;
mod connection;
mod query_builder;
mod value;

pub mod types;

pub use self::backend::{Mysql, MysqlType};
pub use self::connection::MysqlConnection;
pub use self::query_builder::MysqlQueryBuilder;
pub use self::value::{MysqlValue, NumericRepresentation};
