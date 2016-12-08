#[cfg(not(feature = "unstable"))]
mod inner {
    extern crate diesel_codegen_syntex as diesel_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();

        let src = Path::new("tests/lib.in.rs");
        let dst = Path::new(&out_dir).join("lib.rs");

        diesel_codegen::expand(&src, &dst).unwrap();
    }
}

#[cfg(feature = "unstable")]
mod inner {
    pub fn main() {}
}

extern crate diesel;
extern crate dotenv;
use self::diesel::*;
use self::dotenv::dotenv;
use std::io;

#[cfg(feature = "postgres")]
use self::diesel::pg::PgConnection;
#[cfg(feature = "postgres")]
fn connection() -> PgConnection {
    dotenv().ok();
    let database_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run tests");
    PgConnection::establish(&database_url).unwrap()
}

#[cfg(feature = "sqlite")]
use self::diesel::sqlite::SqliteConnection;
#[cfg(feature = "sqlite")]
fn connection() -> SqliteConnection {
    dotenv().ok();
    let database_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run tests");
    SqliteConnection::establish(&database_url).unwrap()
}


#[cfg(feature = "postgres")]
const MIGRATION_SUBDIR: &'static str = "postgresql";

#[cfg(feature = "sqlite")]
const MIGRATION_SUBDIR: &'static str = "sqlite";

fn main() {
    let migrations_dir = migrations::find_migrations_directory().unwrap().join(MIGRATION_SUBDIR);
    migrations::run_pending_migrations_in_directory(&connection(), &migrations_dir, &mut io::sink()).unwrap();
    ::inner::main();
}
