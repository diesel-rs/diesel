#[macro_use]
extern crate diesel;
#[cfg(feature = "dotenv")]
extern crate dotenv;
extern crate itertools;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

mod database_url;
mod migrations;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
mod schema_inference;

pub use self::database_url::extract_database_url;
pub use self::migrations::*;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use self::schema_inference::*;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub enum InferConnection {
    Sqlite(SqliteConnection),
}

#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub enum InferConnection {
    Pg(PgConnection),
}

#[cfg(all(feature = "sqlite", feature = "postgres"))]
pub enum InferConnection {
    Sqlite(SqliteConnection),
    Pg(PgConnection),
}
