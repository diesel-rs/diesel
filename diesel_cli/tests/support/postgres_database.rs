use diesel::expression::sql;
use diesel::types::Bool;
use diesel::{Connection, PgConnection, select, LoadDsl};

pub struct Database {
    url: String
}

impl Database {
    pub fn new(url: &str) -> Self {
        Database {
            url: url.into()
        }
    }

    pub fn create(self) -> Self {
        let (database, postgres_url) = self.split_url();
        let conn = PgConnection::establish(&postgres_url).unwrap();
        conn.execute(&format!(r#"CREATE DATABASE "{}""#, database)).unwrap();
        self
    }

    pub fn exists(&self) -> bool {
        PgConnection::establish(&self.url).is_ok()
    }

    pub fn table_exists(&self, table: &str) -> bool {
        select(sql::<Bool>(&format!("EXISTS \
                (SELECT 1 \
                 FROM information_schema.tables \
                 WHERE table_name = '{}')", table)))
            .get_result(&self.conn()).unwrap()
    }

    pub fn conn(&self) -> PgConnection {
        PgConnection::establish(&self.url)
            .expect(&format!("Failed to open connection to {}", &self.url))
    }

    pub fn execute(&self, command: &str) {
        self.conn().execute(command)
            .expect(&format!("Error executing command {}", command));
    }

    fn split_url(&self) -> (String, String) {
        let mut split: Vec<&str> = self.url.split("/").collect();
        let default_database = "postgres";
        let database_name_with_arguments: Vec<&str> = split.pop().unwrap().split('?').collect();
        let database = database_name_with_arguments[0];
        let postgres_url;
        match database_name_with_arguments.len() {
            2 => {
                let args : &str = database_name_with_arguments[1];
                postgres_url = format!("{}/{}?{}", split.join("/"), default_database, args);
            },
            _ => postgres_url = format!("{}/{}", split.join("/"), default_database)
        }
        (database.into(), postgres_url)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let (database, postgres_url) = self.split_url();
        let conn = try_drop!(PgConnection::establish(&postgres_url), "Couldn't connect to database");
        conn.silence_notices(|| {
            try_drop!(conn.execute(&format!(r#"DROP DATABASE IF EXISTS "{}""#, database)), "Couldn't drop database");
        });
    }
}
