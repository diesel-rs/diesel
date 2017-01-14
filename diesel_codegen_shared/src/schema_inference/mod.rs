mod data_structures;
#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

use diesel::Connection;
use diesel::result::Error::NotFound;
#[cfg(feature = "postgres")]
use itertools::Itertools;
use std::error::Error;

use InferConnection;
use database_url::extract_database_url;
pub use self::data_structures::{ColumnInformation, ColumnType};
#[cfg(feature = "postgres")]
pub use self::data_structures::EnumInformation;
#[cfg(feature = "postgres")]
pub use self::pg::{camel_cased, canonicalize_pg_type_name};

pub fn load_table_names(database_url: &str, schema_name: Option<&str>)
    -> Result<Vec<String>, Box<Error>>
{
    let connection = try!(establish_connection(database_url));

    match connection {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(c) => sqlite::load_table_names(&c, schema_name),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(c) => pg::load_table_names(&c, schema_name),
    }
}

pub fn establish_connection(database_url: &str) -> Result<InferConnection, Box<Error>> {
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

pub fn get_table_data(conn: &InferConnection, table_name: &str)
    -> Result<Vec<ColumnInformation>, Box<Error>>
{
    let column_info = match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref c) => sqlite::get_table_data(c, table_name),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref c) => pg::get_table_data(c, table_name),
    };
    if let Err(NotFound) = column_info {
        Err(format!("no table exists named {}", table_name).into())
    } else {
        column_info.map_err(Into::into)
    }
}

pub fn determine_column_type(
    extra_types_module: Option<&str>,
    attr: &ColumnInformation,
    conn: &InferConnection,
) -> Result<ColumnType, Box<Error>> {
    match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(_) => sqlite::determine_column_type(attr),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(_) => pg::determine_column_type(extra_types_module, attr),
    }
}

pub fn get_primary_keys(
    conn: &InferConnection,
    table_name: &str,
) -> Result<Vec<String>, Box<Error>> {
    let primary_keys = try!(match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref c) => sqlite::get_primary_keys(c, table_name),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref c) => pg::get_primary_keys(c, table_name),
    });
    if primary_keys.is_empty() {
        Err(format!("Diesel only supports tables with primary keys. \
                    Table {} has no primary key", table_name).into())
    } else if primary_keys.len() > 4 {
        Err(format!("Diesel does not currently support tables with \
                     primary keys consisting of more than 4 columns. \
                     Table {} has {} columns in its primary key. \
                     Please open an issue and we will increase the \
                     limit.", table_name, primary_keys.len()).into())
    } else {
        Ok(primary_keys)
    }
}

#[cfg(feature = "postgres")]
pub fn get_enum_information(conn: &InferConnection, schema_name: Option<&str>)
                            -> Result<Vec<EnumInformation>, Box<Error>> {
    let rows = try!(match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(_) =>
            panic!("Diesel does not support inferring enum types from SQLite"),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref c) => pg::load_enum_info(c, schema_name),
    });
    let mut enum_information = vec!();
    for (oid, grouped) in rows.into_iter().group_by(|&(_, _, oid)| oid).into_iter() {
        let mut grouped = grouped.peekable();
        let type_name = match grouped.peek() {
            Some(&(ref type_name, _, _)) => type_name.clone(),
            None => panic!("enum must have at least one variant"),
        };
        let variants = grouped.map(|(_, ref variant_name, _)| variant_name.clone()).collect();
        enum_information.push(EnumInformation {
            type_name: type_name,
            variants: variants,
            oid: oid,
            array_oid: oid,  // TODO: fix this.
        });
    }
    Ok(enum_information)
}
