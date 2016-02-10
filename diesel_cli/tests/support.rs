use dotenv::dotenv;

use rand::{thread_rng, Rng};

use diesel::expression::sql;
use diesel::types::Bool;
use diesel::{Connection, LoadDsl, select};

use std::path::PathBuf;
use std::{env, fs};

use tempdir::TempDir;

#[cfg(feature = "postgres")]
pub type TestConnection = ::diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
pub type TestConnection = ::diesel::sqlite::SqliteConnection;

pub struct TestDatabase {
    pub database_url: String
}

impl Drop for TestDatabase {
    #[cfg(feature = "postgres")]
    fn drop(&mut self) {
        let mut split: Vec<&str> = self.database_url.split("/").collect();
        // Remove the user's database from the system url
        let database = split.pop().unwrap();
        let system_url = split.join("/");

        let conn = TestConnection::establish(&system_url).unwrap();
        conn.silence_notices(|| {
            conn.execute(&format!("DROP DATABASE IF EXISTS {}", database)).unwrap();
        });
    }

    #[cfg(feature = "sqlite")]
    fn drop(&mut self) {
        // TempDir takes care of cleaing up the entire directory holding our database
    }
}

impl TestDatabase {
    #[cfg(feature = "postgres")]
    pub fn new(database: &String, _: &PathBuf) -> Self {
        dotenv().ok();
        let postgres_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set to run diesel_cli tests");
        let mut split: Vec<&str> = postgres_url.split("/").collect();
        // Remove the user's database from the system url
        split.pop().unwrap();
        let system_url = split.join("/");

        let conn = TestConnection::establish(&system_url).unwrap();
        conn.execute(&format!("CREATE DATABASE {};", database)).unwrap();

        TestDatabase {
            database_url: format!("{}/{}", system_url, database)
        }
    }

    #[cfg(feature = "sqlite")]
    pub fn new(database: &String, root_dir: &PathBuf) -> Self {
        let database_path = root_dir.join(database);
        fs::File::create(&database_path).unwrap();

        TestDatabase {
            database_url: database_path.to_str().unwrap().to_owned(),
        }
    }
}

pub struct TestEnvironment {
    pub root_dir: TempDir,
    pub identifier: String,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new("diesel").unwrap();
        // This needs to be lowercase because postgres will only create
        // databases with all lowercase letters, but we will happily try to
        // connect to a database with uppercase letters.
        let rstr: String = (0..8).map(|_|
                                      thread_rng().gen_range(b'a', b'z') as char
                                      ).collect();

        TestEnvironment {
            root_dir: temp_dir,
            identifier: rstr,
        }
    }

    pub fn root_path(&self) -> PathBuf {
        self.root_dir.path().canonicalize().unwrap()
    }
}

#[cfg(feature = "postgres")]
pub fn table_exists(database_url: &String, table: &str) -> bool {
    let conn = TestConnection::establish(database_url).unwrap();
    select(sql::<Bool>(&format!("EXISTS \
                    (SELECT 1 \
                     FROM information_schema.tables \
                     WHERE table_name = '{}')", table)))
        .get_result(&conn).unwrap()
}

#[cfg(feature = "sqlite")]
pub fn table_exists(database_url: &String, table: &str) -> bool {
    let conn = TestConnection::establish(database_url).unwrap();
    select(sql::<Bool>(&format!("EXISTS \
                    (SELECT 1 \
                     FROM sqlite_master \
                     WHERE type = 'table' \
                     AND name = '{}')", table)))
        .get_result(&conn).unwrap()
}
