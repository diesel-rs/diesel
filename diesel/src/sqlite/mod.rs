//! Provides types and functions related to working with SQLite
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! SQLite, you may need to work with this module directly.

mod backend;
mod connection;
mod types;

pub mod query_builder;

pub use self::backend::{Sqlite, SqliteType};
pub use self::connection::SqliteConnection;
pub use self::query_builder::SqliteQueryBuilder;
