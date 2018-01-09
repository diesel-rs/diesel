use diesel::connection::SimpleConnection;
use diesel::dsl::sql;
use diesel::sql_types::Bool;
use diesel::{select, Connection, PgConnection, RunQueryDsl};

pub struct Database {
    url: String,
}

impl Database {
    pub fn new(url: &str) -> Self {
        Database { url: url.into() }
    }

    pub fn create(self) -> Self {
        let (database, postgres_url) = self.split_url();
        let conn = PgConnection::establish(&postgres_url).unwrap();
        conn.execute(&format!(r#"CREATE DATABASE "{}""#, database))
            .unwrap();
        self
    }

    pub fn exists(&self) -> bool {
        PgConnection::establish(&self.url).is_ok()
    }

    pub fn table_exists(&self, table: &str) -> bool {
        select(sql::<Bool>(&format!(
            "EXISTS \
             (SELECT 1 \
             FROM information_schema.tables \
             WHERE table_name = '{}')",
            table
        ))).get_result(&self.conn())
            .unwrap()
    }

    pub fn conn(&self) -> PgConnection {
        PgConnection::establish(&self.url)
            .expect(&format!("Failed to open connection to {}", &self.url))
    }

    pub fn execute(&self, command: &str) {
        self.conn()
            .batch_execute(command)
            .expect(&format!("Error executing command {}", command));
    }

    fn split_url(&self) -> (String, String) {
        let mut split: Vec<&str> = self.url.split("/").collect();
        let database = split.pop().unwrap();
        let postgres_url = format!("{}/{}", split.join("/"), "postgres");
        (database.into(), postgres_url)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let (database, postgres_url) = self.split_url();
        let conn = try_drop!(
            PgConnection::establish(&postgres_url),
            "Couldn't connect to database"
        );
        try_drop!(
            conn.execute(&format!(r#"DROP DATABASE IF EXISTS "{}""#, database)),
            "Couldn't drop database"
        );
    }
}
