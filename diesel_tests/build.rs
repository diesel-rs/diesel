extern crate diesel;
extern crate diesel_migrations as migrations;
extern crate dotenv;
use self::diesel::*;
use self::dotenv::dotenv;
use std::{env, io};

#[cfg(not(any(feature = "mysql", feature = "sqlite", feature = "postgres")))]
compile_error!(
    "At least one backend must be used to test this crate.\n \
     Pass argument `--features \"<backend>\"` with one or more of the following backends, \
     'mysql', 'postgres', or 'sqlite'. \n\n \
     ex. cargo test --features \"mysql postgres sqlite\"\n"
);

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
const MIGRATION_SUBDIR: &str = "postgresql";

#[cfg(feature = "sqlite")]
const MIGRATION_SUBDIR: &str = "sqlite";

#[cfg(feature = "mysql")]
const MIGRATION_SUBDIR: &str = "mysql";

fn database_url_from_env(backend_specific_env_var: &str) -> String {
    dotenv().ok();
    match env::var(backend_specific_env_var) {
        Ok(val) => {
            println!(r#"cargo:rustc-cfg=feature="backend_specific_database_url""#);
            val
        }
        _ => env::var("DATABASE_URL").expect("DATABASE_URL must be set in order to run tests"),
    }
}

fn main() {
    let migrations_dir = migrations::find_migrations_directory()
        .unwrap()
        .join(MIGRATION_SUBDIR);
    println!("cargo:rerun-if-changed={}", migrations_dir.display());
    migrations::run_pending_migrations_in_directory(
        &connection(),
        &migrations_dir,
        &mut io::sink(),
    )
    .unwrap();
}
