use std::error::Error;

use diesel::result::Error::NotFound;

use super::data_structures::*;
use super::table_data::*;
use crate::database::InferConnection;
use crate::print_schema::ColumnSorting;

static RESERVED_NAMES: &[&str] = &[
    "abstract", "alignof", "as", "become", "box", "break", "const", "continue", "crate", "do",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "offsetof", "override", "priv", "proc", "pub", "pure",
    "ref", "return", "Self", "self", "sizeof", "static", "struct", "super", "trait", "true",
    "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield", "bool",
    "columns",
];

fn is_reserved_name(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
        || (
            // Names ending in an underscore are not considered reserved so that we
            // can always just append an underscore to generate an unreserved name.
            name.starts_with("__") && !name.ends_with('_')
        )
}

fn contains_unmappable_chars(name: &str) -> bool {
    // Rust identifier names are restricted to [a-zA-Z0-9_].
    !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn rust_name_for_sql_name(sql_name: &str) -> String {
    if is_reserved_name(sql_name) {
        format!("{}_", sql_name)
    } else if contains_unmappable_chars(sql_name) {
        // Map each non-alphanumeric character ([^a-zA-Z0-9]) to an underscore.
        let mut rust_name: String = sql_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();

        // Iteratively remove adjoining underscores ("__").
        let mut last_len = rust_name.len();
        'remove_adjoining: loop {
            rust_name = rust_name.replace("__", "_");
            if rust_name.len() == last_len {
                // No more underscore pairs left.
                break 'remove_adjoining;
            }
            last_len = rust_name.len();
        }

        rust_name
    } else {
        sql_name.to_string()
    }
}

pub fn load_table_names(
    database_url: &str,
    schema_name: Option<&str>,
) -> Result<Vec<TableName>, Box<dyn Error + Send + Sync + 'static>> {
    let mut connection = InferConnection::establish(database_url)?;

    match connection {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => super::sqlite::load_table_names(c, schema_name),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => {
            super::information_schema::load_table_names(c, schema_name)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => {
            super::information_schema::load_table_names(c, schema_name)
        }
    }
}

fn get_column_information(
    conn: &mut InferConnection,
    table: &TableName,
    column_sorting: &ColumnSorting,
) -> Result<Vec<ColumnInformation>, Box<dyn Error + Send + Sync + 'static>> {
    let column_info = match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => {
            super::sqlite::get_table_data(c, table, column_sorting)
        }
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => {
            super::information_schema::get_table_data(c, table, column_sorting)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => {
            super::information_schema::get_table_data(c, table, column_sorting)
        }
    };
    if let Err(NotFound) = column_info {
        Err(format!("no table exists named {}", table.to_string()).into())
    } else {
        column_info.map_err(Into::into)
    }
}

fn determine_column_type(
    attr: &ColumnInformation,
    conn: &mut InferConnection,
) -> Result<ColumnType, Box<dyn Error + Send + Sync + 'static>> {
    match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(_) => super::sqlite::determine_column_type(attr),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut conn) => {
            use crate::infer_schema_internals::information_schema::UsesInformationSchema;

            super::pg::determine_column_type(attr, diesel::pg::Pg::default_schema(conn)?)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(_) => super::mysql::determine_column_type(attr),
    }
}

pub(crate) fn get_primary_keys(
    conn: &mut InferConnection,
    table: &TableName,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
    let primary_keys: Vec<String> = match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => super::sqlite::get_primary_keys(c, table),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => super::information_schema::get_primary_keys(c, table),
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => super::information_schema::get_primary_keys(c, table),
    }?;
    if primary_keys.is_empty() {
        Err(format!(
            "Diesel only supports tables with primary keys. \
             Table {} has no primary key",
            table.to_string()
        )
        .into())
    } else {
        Ok(primary_keys)
    }
}

pub fn load_foreign_key_constraints(
    database_url: &str,
    schema_name: Option<&str>,
) -> Result<Vec<ForeignKeyConstraint>, Box<dyn Error + Send + Sync + 'static>> {
    let mut connection = InferConnection::establish(database_url)?;

    let constraints = match connection {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => {
            super::sqlite::load_foreign_key_constraints(c, schema_name)
        }
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => {
            super::information_schema::load_foreign_key_constraints(c, schema_name)
                .map_err(Into::into)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => {
            super::mysql::load_foreign_key_constraints(c, schema_name).map_err(Into::into)
        }
    };

    constraints.map(|mut ct| {
        ct.sort();
        ct.iter_mut().for_each(|foreign_key_constraint| {
            if is_reserved_name(&foreign_key_constraint.foreign_key_rust_name) {
                foreign_key_constraint.foreign_key_rust_name =
                    format!("{}_", foreign_key_constraint.foreign_key_rust_name);
            }
        });
        ct
    })
}

macro_rules! doc_comment {
    ($($token:tt)*) => {
        format!($($token)*)
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
    };
}

pub fn load_table_data(
    database_url: &str,
    name: TableName,
    column_sorting: &ColumnSorting,
) -> Result<TableData, Box<dyn Error + Send + Sync + 'static>> {
    let mut connection = InferConnection::establish(database_url)?;
    let docs = doc_comment!(
        "Representation of the `{}` table.

        (Automatically generated by Diesel.)",
        name.full_sql_name(),
    );
    let primary_key = get_primary_keys(&mut connection, &name)?;
    let primary_key = primary_key
        .iter()
        .map(|k| rust_name_for_sql_name(&k))
        .collect();

    let column_data = get_column_information(&mut connection, &name, column_sorting)?
        .into_iter()
        .map(|c| {
            let ty = determine_column_type(&c, &mut connection)?;
            let rust_name = rust_name_for_sql_name(&c.column_name);

            Ok(ColumnDefinition {
                docs: doc_comment!(
                    "The `{}` column of the `{}` table.

                    Its SQL type is `{}`.

                    (Automatically generated by Diesel.)",
                    c.column_name,
                    name.full_sql_name(),
                    ty
                ),
                sql_name: c.column_name,
                ty,
                rust_name,
            })
        })
        .collect::<Result<_, Box<dyn Error + Send + Sync + 'static>>>()?;

    Ok(TableData {
        name,
        primary_key,
        column_data,
        docs,
    })
}
