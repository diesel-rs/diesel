use std::collections::HashMap;

use diesel::result::Error::NotFound;

use super::table_data::*;
use super::{data_structures::*, SchemaResolverImpl};

use crate::config::{Filtering, PrintSchema};

use crate::database::InferConnection;
use crate::print_schema::{ColumnSorting, DocConfig};

static RESERVED_NAMES: &[&str] = &[
    "abstract",
    "alignof",
    "as",
    "become",
    "box",
    "break",
    "const",
    "continue",
    "crate",
    "do",
    "else",
    "enum",
    "extern",
    "false",
    "final",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "macro",
    "match",
    "mod",
    "move",
    "mut",
    "offsetof",
    "override",
    "priv",
    "proc",
    "pub",
    "pure",
    "ref",
    "return",
    "Self",
    "self",
    "sizeof",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "typeof",
    "unsafe",
    "unsized",
    "use",
    "virtual",
    "where",
    "while",
    "yield",
    "bool",
    "table",
    "columns",
    "is_nullable",
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
        format!("{sql_name}_")
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

#[tracing::instrument(skip(connection))]
pub fn load_table_names(
    connection: &mut InferConnection,
    schema_name: Option<&str>,
) -> Result<Vec<(SupportedQueryRelationStructures, TableName)>, crate::errors::Error> {
    let tables = match connection {
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
    }?;

    tracing::info!(?tables, "Loaded tables");
    Ok(tables)
}

pub fn filter_column_structure(
    table_names: &[(SupportedQueryRelationStructures, TableName)],
    structure: SupportedQueryRelationStructures,
) -> Vec<TableName> {
    table_names
        .iter()
        .filter_map(|(s, t)| if *s == structure { Some(t) } else { None })
        .cloned()
        .collect()
}

pub fn filter_table_names(
    table_names: &[(SupportedQueryRelationStructures, TableName)],
    table_filter: &Filtering,
    include_views: bool,
) -> Vec<(SupportedQueryRelationStructures, TableName)> {
    table_names
        .iter()
        .filter(|(a, _)| include_views || matches!(a, SupportedQueryRelationStructures::Table))
        .filter(|(_, t)| !table_filter.should_ignore_table(t))
        .cloned()
        .collect::<_>()
}

#[tracing::instrument(skip(conn))]
fn get_table_comment(
    conn: &mut InferConnection,
    table: &TableName,
) -> Result<Option<String>, crate::errors::Error> {
    let table_comment = match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(_) => Ok(None),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => super::pg::get_table_comment(c, table),
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => super::mysql::get_table_comment(c, table),
    };
    if let Err(NotFound) = table_comment {
        Err(crate::errors::Error::NoTableFound(table.clone()))
    } else {
        let table_comment = table_comment?;
        tracing::info!(?table_comment, "Load table comments for {table}");
        Ok(table_comment)
    }
}

fn get_column_information(
    conn: &mut InferConnection,
    table: &TableName,
    column_sorting: &ColumnSorting,
    pg_domains_as_custom_types: &[&regex::Regex],
    kind: SupportedQueryRelationStructures,
) -> Result<Vec<ColumnInformation>, crate::errors::Error> {
    #[cfg(not(feature = "postgres"))]
    let _ = pg_domains_as_custom_types;
    #[cfg(not(feature = "sqlite"))]
    let _ = kind;

    let column_info = match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => {
            super::sqlite::get_table_data(c, table, column_sorting, kind)
        }
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => {
            super::pg::get_table_data(c, table, column_sorting, pg_domains_as_custom_types)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => super::mysql::get_table_data(c, table, column_sorting),
    };
    if let Err(NotFound) = column_info {
        Err(crate::errors::Error::NoTableFound(table.clone()))
    } else {
        let column_info = column_info?;
        tracing::info!(?column_info, "Load column information for table {table}");
        Ok(column_info)
    }
}

fn determine_column_type(
    attr: &ColumnInformation,
    conn: &mut InferConnection,
    #[allow(unused_variables)] table: &TableName,
    #[allow(unused_variables)] primary_keys: Option<&[String]>,
    #[allow(unused_variables)] foreign_keys: &HashMap<String, ForeignKeyConstraint>,
    #[allow(unused_variables)] config: &PrintSchema,
) -> Result<ColumnType, crate::errors::Error> {
    match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut conn) => super::sqlite::determine_column_type(
            conn,
            attr,
            table,
            primary_keys,
            foreign_keys,
            config,
        ),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut conn) => {
            use crate::infer_schema_internals::information_schema::DefaultSchema;

            super::pg::determine_column_type(attr, diesel::pg::Pg::default_schema(conn)?)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(_) => super::mysql::determine_column_type(attr),
    }
}

#[tracing::instrument(skip(conn))]
pub(crate) fn get_primary_keys(
    conn: &mut InferConnection,
    table: &TableName,
) -> Result<Vec<String>, crate::errors::Error> {
    let primary_keys: Vec<String> = match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => super::sqlite::get_primary_keys(c, table),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => super::information_schema::get_primary_keys(c, table),
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => super::information_schema::get_primary_keys(c, table),
    }?;
    if primary_keys.is_empty() {
        Err(crate::errors::Error::NoPrimaryKeyFound(table.clone()))
    } else {
        tracing::info!(?primary_keys, "Load primary keys for table {table}");
        Ok(primary_keys)
    }
}

#[tracing::instrument(skip(connection))]
pub fn load_foreign_key_constraints(
    connection: &mut InferConnection,
    schema_name: Option<&str>,
) -> Result<Vec<ForeignKeyConstraint>, crate::errors::Error> {
    let constraints = match connection {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut c) => {
            super::sqlite::load_foreign_key_constraints(c, schema_name)
        }
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut c) => {
            super::pg::load_foreign_key_constraints(c, schema_name).map_err(Into::into)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut c) => {
            super::mysql::load_foreign_key_constraints(c, schema_name).map_err(Into::into)
        }
    };

    constraints.map(|mut ct| {
        ct.sort();
        ct.iter_mut().for_each(|foreign_key_constraint| {
            for name in &mut foreign_key_constraint.foreign_key_columns_rust {
                if is_reserved_name(name) {
                    *name = format!("{name}_");
                }
            }
        });
        tracing::info!(?ct, "Loaded foreign key constraints");
        ct
    })
}

#[tracing::instrument(skip(connection))]
fn load_column_structure_data(
    connection: &mut InferConnection,
    name: &TableName,
    config: &PrintSchema,
    primary_key: Option<&[String]>,
    kind: SupportedQueryRelationStructures,
) -> Result<(Option<String>, Vec<ColumnDefinition>), crate::errors::Error> {
    // No point in loading table comments if they are not going to be displayed
    let table_comment = match config.with_docs {
        DocConfig::NoDocComments => None,
        DocConfig::OnlyDatabaseComments
        | DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment => {
            get_table_comment(connection, name)?
        }
    };

    let foreign_keys = load_foreign_key_constraints(connection, name.schema.as_deref())?
        .into_iter()
        .filter_map(|c| {
            if c.child_table == *name && c.foreign_key_columns.len() == 1 {
                Some((c.foreign_key_columns_rust[0].clone(), c))
            } else {
                None
            }
        })
        .collect();

    let pg_domains_as_custom_types = config
        .pg_domains_as_custom_types
        .iter()
        .map(|regex| regex as &regex::Regex)
        .collect::<Vec<_>>();

    get_column_information(
        connection,
        name,
        &config.column_sorting,
        &pg_domains_as_custom_types,
        kind,
    )?
    .into_iter()
    .map(|c| {
        let ty = determine_column_type(&c, connection, name, primary_key, &foreign_keys, config)?;

        let ColumnInformation {
            column_name,
            comment,
            ..
        } = c;
        let rust_name = rust_name_for_sql_name(&column_name);

        Ok(ColumnDefinition {
            sql_name: column_name,
            ty,
            rust_name,
            comment,
        })
    })
    .collect::<Result<_, crate::errors::Error>>()
    .map(|data| (table_comment, data))
}

#[tracing::instrument(skip(connection))]
pub fn load_table_data(
    connection: &mut InferConnection,
    name: TableName,
    config: &PrintSchema,
    tpe: SupportedQueryRelationStructures,
) -> Result<TableData, crate::errors::Error> {
    let primary_key = match tpe {
        SupportedQueryRelationStructures::Table => get_primary_keys(connection, &name)?,
        SupportedQueryRelationStructures::View => Vec::new(),
    };
    let (table_comment, column_data) =
        load_column_structure_data(connection, &name, config, Some(&primary_key), tpe)?;
    let primary_key = primary_key
        .iter()
        .map(|k| rust_name_for_sql_name(k))
        .collect::<Vec<_>>();
    Ok(TableData {
        name,
        primary_key,
        column_data,
        comment: table_comment,
    })
}

#[tracing::instrument(skip(resolver))]
pub fn load_view_data(
    resolver: &mut SchemaResolverImpl,
    name: TableName,
) -> Result<ViewData, crate::errors::Error> {
    let (table_comment, mut column_data) = load_column_structure_data(
        resolver.connection,
        &name,
        resolver.config,
        None,
        SupportedQueryRelationStructures::View,
    )?;
    let sql_definition = load_view_sql_definition(resolver.connection, &name)?;
    if resolver.config.experimental_infer_nullable_for_views {
        tracing::debug!("Infer nullability for view fields");
        match diesel_infer_query::parse_view_def(&sql_definition) {
            Ok(mut data) => {
                if data
                    .resolve_references(resolver)
                    .map_err(|e| {
                        tracing::debug!(view = %name, ?data, ?e, "Failed to resolve references");
                        e
                    })
                    .is_ok()
                {
                    tracing::debug!(view = %name, ?data, "Inferred data");
                    if data.field_count() == column_data.len() {
                        for (column_data, is_nullable) in column_data
                            .iter_mut()
                            .zip(data.infer_nullability(resolver)?)
                        {
                            tracing::debug!(view = %name, field = %column_data.rust_name, ?is_nullable, "Correct field nullablility");
                            if let Some(is_nullable) = is_nullable {
                                column_data.ty.is_nullable = is_nullable;
                            }
                        }
                    } else {
                        tracing::warn!(view = %name, ?data, ?column_data, "Field count mismatch between what the database returned and what we inferred");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(view = %name, error = %e, "Failed to infer nullablity for view fields")
            }
        }
    }
    Ok(ViewData {
        name,
        column_data,
        comment: table_comment,
        sql_definition,
    })
}

fn load_view_sql_definition(
    connection: &mut InferConnection,
    name: &TableName,
) -> Result<String, crate::errors::Error> {
    match connection {
        #[cfg(feature = "postgres")]
        InferConnection::Pg(pg_connection) => Ok(
            super::information_schema::load_view_sql_definition(pg_connection, name)?,
        ),
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(sqlite_connection) => {
            super::sqlite::load_view_sql_definition(sqlite_connection, name)
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(mysql_connection) => Ok(
            super::information_schema::load_view_sql_definition(mysql_connection, name)?,
        ),
    }
}
