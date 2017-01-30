#[macro_use]
extern crate diesel;
#[cfg(feature = "dotenv")]
extern crate dotenv;

mod database_url;
mod migrations;

pub use self::database_url::extract_database_url;
pub use self::migrations::*;
