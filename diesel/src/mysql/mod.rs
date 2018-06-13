//! Provides types and functions related to working with MySQL
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MySQL, you may need to work with this module directly.

mod backend;
mod bind_collector;
mod connection;

mod query_builder;
pub mod types;

pub use self::backend::{Mysql, MysqlType};
pub use self::connection::MysqlConnection;
pub use self::query_builder::MysqlQueryBuilder;
