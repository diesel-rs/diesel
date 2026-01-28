#[allow(unused)]
macro_rules! try_drop {
    ($e:expr, $msg:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                if ::std::thread::panicking() {
                    eprintln!("{}: {:?}", $msg, e);
                    return;
                } else {
                    panic!("{}: {:?}", $msg, e);
                }
            }
        }
    };
}

mod command;
mod project_builder;

#[cfg_attr(feature = "sqlite", path = "sqlite_database.rs")]
#[cfg_attr(feature = "postgres", path = "postgres_database.rs")]
#[cfg_attr(feature = "mysql", path = "mysql_database.rs")]
pub mod database;

#[cfg(rustfmt)]
mod mysql_database;
#[cfg(rustfmt)]
mod postgres_database;
#[cfg(rustfmt)]
mod sqlite_database;

pub use self::project_builder::{project, Project};

pub fn database(url: &str) -> database::Database {
    database::Database::new(url)
}
