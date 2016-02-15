mod command;
mod project_builder;

#[cfg(feature = "sqlite")]
#[path="sqlite_database.rs"]
pub mod database;

#[cfg(feature = "postgres")]
#[path="postgres_database.rs"]
pub mod database;

pub use self::project_builder::project;

pub fn database(url: &str) -> database::Database {
    database::Database::new(url)
}
