#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

use diesel::Connection;
use std::error::Error;

use database_url::extract_database_url;
use InferConnection;

pub fn load_table_names(database_url: &str) -> Result<Vec<String>, Box<Error>> {
    let connection = try!(establish_connection(database_url));

    match connection {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(c) => sqlite::load_table_names(&c),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(c) => pg::load_table_names(&c),
    }
}

fn establish_connection(database_url: &str) -> Result<InferConnection, Box<Error>> {
    let database_url = try!(extract_database_url(database_url));
    match database_url {
        #[cfg(feature = "postgres")]
        _ if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") => {
            establish_real_connection(&database_url).map(InferConnection::Pg)
        }
        #[cfg(feature = "sqlite")]
        _ => establish_real_connection(&database_url).map(InferConnection::Sqlite),
        #[cfg(not(feature = "sqlite"))]
        _ => {
            Err(format!(
                "{} is not a valid PG database URL. \
                It must start with postgres:// or postgresql://",
                database_url,
            ).into())
        }
    }
}

fn establish_real_connection<Conn>(database_url: &str) -> Result<Conn, Box<Error>> where
    Conn: Connection,
{
    Conn::establish(database_url).map_err(|error| {
        format!(
            "Failed to establish a database connection at {}. Error: {:?}",
            database_url,
            error,
        ).into()
    })
}

