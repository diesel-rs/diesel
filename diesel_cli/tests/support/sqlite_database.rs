use diesel::connection::SimpleConnection;
use diesel::dsl::sql;
use diesel::sqlite::SqliteConnection;
use diesel::sql_types::Bool;
use diesel::{select, Connection, RunQueryDsl};

use std::fs;

pub struct Database {
    url: String,
}

impl Database {
    pub fn new(url: &str) -> Self {
        Database { url: url.into() }
    }

    pub fn create(self) -> Self {
        fs::File::create(&self.url).expect(&format!("Error creating database {}", &self.url));
        self
    }

    pub fn exists(&self) -> bool {
        use std::path::Path;
        Path::new(&self.url).exists()
    }

    pub fn table_exists(&self, table: &str) -> bool {
        select(sql::<Bool>(&format!(
            "EXISTS \
             (SELECT 1 \
             FROM sqlite_master \
             WHERE type = 'table' \
             AND name = '{}')",
            table
        ))).get_result(&self.conn())
            .unwrap()
    }

    pub fn conn(&self) -> SqliteConnection {
        SqliteConnection::establish(&self.url)
            .expect(&format!("Failed to open connection to {}", &self.url))
    }

    pub fn execute(&self, command: &str) {
        self.conn()
            .batch_execute(command)
            .expect(&format!("Error executing command {}", command));
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        // TempDir's Drop implementation takes care of this for us.
    }
}
