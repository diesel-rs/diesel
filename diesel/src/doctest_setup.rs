extern crate dotenv;

use diesel::backend;
use diesel::persistable::{InsertValues};
use diesel::prelude::*;
use diesel::query_builder::{QueryBuilder, BuildQueryResult};
use self::dotenv::dotenv;

#[cfg(feature = "postgres")]
type DB = diesel::pg::Pg;

#[cfg(feature = "postgres")]
fn connection_no_data() -> diesel::pg::PgConnection {
    dotenv().ok();

    let connection_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in order to run tests");
    let connection = diesel::pg::PgConnection::establish(&connection_url).unwrap();
    connection.begin_test_transaction().unwrap();
    connection.execute("DROP TABLE IF EXISTS users").unwrap();

    connection
}

#[cfg(feature = "postgres")]
fn establish_connection() -> diesel::pg::PgConnection {
    let connection = connection_no_data();

    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL
    )").unwrap();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();

    connection
}

#[cfg(all(not(feature = "postgres"), feature = "sqlite"))]
type DB = diesel::sqlite::Sqlite;

#[cfg(all(not(feature = "postgres"), feature = "sqlite"))]
fn connection_no_data() -> diesel::sqlite::SqliteConnection {
    diesel::sqlite::SqliteConnection::establish(":memory:").unwrap()
}

#[cfg(all(not(feature = "postgres"), feature = "sqlite"))]
fn establish_connection() -> diesel::sqlite::SqliteConnection {
    let connection = connection_no_data();

    connection.execute("CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name VARCHAR NOT NULL
    )").unwrap();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();

    connection
}

#[derive(Clone)]
struct NewUser {
    name: String,
}

struct NewUserValues {
    name: String,
}

impl<DB> InsertValues<DB> for NewUserValues where
    DB: diesel::backend::Backend,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("name");
        Ok(())
    }

    fn values_clause(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(&format!("('{}')", self.name));
        Ok(())
    }

    fn values_bind_params(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }
}

impl<'a, DB> Insertable<users::table, DB> for &'a NewUser where
    DB: diesel::backend::Backend,
{
    type Values = NewUserValues;

    fn values(self) -> Self::Values {
        NewUserValues {
            name: self.name.clone(),
        }
    }
}
