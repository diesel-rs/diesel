extern crate diesel;
extern crate dotenv;
use self::diesel::*;
use self::dotenv::dotenv;
use std::{io, env};

#[cfg(feature = "postgres")]
fn connection() -> PgConnection {
    let database_url = database_url_from_env("PG_DATABASE_URL");
    PgConnection::establish(&database_url).unwrap()
}

#[cfg(feature = "sqlite")]
fn connection() -> SqliteConnection {
    let database_url = database_url_from_env("SQLITE_DATABASE_URL");
    SqliteConnection::establish(&database_url).unwrap()
}

#[cfg(feature = "mysql")]
fn connection() -> MysqlConnection {
    let database_url = database_url_from_env("MYSQL_DATABASE_URL");
    MysqlConnection::establish(&database_url).unwrap()
}

#[cfg(feature = "postgres")]
const MIGRATION_SUBDIR: &'static str = "postgresql";

#[cfg(feature = "sqlite")]
const MIGRATION_SUBDIR: &'static str = "sqlite";

#[cfg(feature = "mysql")]
const MIGRATION_SUBDIR: &'static str = "mysql";

fn database_url_from_env(backend_specific_env_var: &str) -> String {
    dotenv().ok();
    match env::var(backend_specific_env_var) {
        Ok(val) => {
            println!(r#"cargo:rustc-cfg=feature="backend_specific_database_url""#);
            val
        }
        _ => {
            env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set in order to run tests")
        }
    }
}

fn main() {
    let migrations_dir = migrations::find_migrations_directory().unwrap().join(MIGRATION_SUBDIR);
    migrations::run_pending_migrations_in_directory(&connection(), &migrations_dir, &mut io::sink()).unwrap();
}
