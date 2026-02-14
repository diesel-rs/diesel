use diesel::QueryResult;
use diesel::backend::Backend;
use diesel::query_builder::QueryBuilder;
use diesel_table_macro_syntax::{ColumnDef, TableDecl, ViewDecl};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use syn::visit::Visit;

use crate::config::PrintSchema;
use crate::database::InferConnection;
use crate::infer_schema_internals::{
    ColumnDefinition, ColumnType, ForeignKeyConstraint, SupportedQueryRelationStructures,
    TableData, TableName, filter_table_names, load_table_names,
};
use crate::print_schema::{ColumnSorting, DocConfig};

fn compatible_type_list() -> HashMap<&'static str, Vec<&'static str>> {
    let mut map = HashMap::new();
    map.insert("integer", vec!["int4"]);
    map.insert("bigint", vec!["int8"]);
    map.insert("smallint", vec!["int2"]);
    map.insert("text", vec!["varchar"]);
    map
}

#[tracing::instrument]
pub fn generate_sql_based_on_diff_schema(
    mut config: PrintSchema,
    database_url: Option<String>,
    schema_file_path: &Path,
    table_name: Vec<String>,
    only_tables: Vec<bool>,
    except_tables: Vec<bool>,
) -> Result<(String, String), crate::errors::Error> {
    config.set_filter(table_name, only_tables, except_tables)?;

    let project_root = crate::find_project_root()?;

    let schema_path = project_root.join(schema_file_path);
    let content = std::fs::read_to_string(&schema_path)
        .map_err(|e| crate::errors::Error::IoError(e, Some(schema_path.clone())))?;

    let syn_file = syn::parse_file(&content)?;

    let mut tables_from_schema = SchemaCollector::default();

    tables_from_schema.visit_file(&syn_file);
    let mut conn = InferConnection::from_maybe_url(database_url)?;

    let foreign_keys =
        crate::infer_schema_internals::load_foreign_key_constraints(&mut conn, None)?;
    let foreign_key_map =
        foreign_keys
            .into_iter()
            .fold(HashMap::<_, Vec<_>>::new(), |mut acc, t| {
                acc.entry(t.child_table.rust_name.clone())
                    .or_default()
                    .push(t);
                acc
            });

    let mut expected_fk_map = tables_from_schema.joinable.into_iter().try_fold(
        HashMap::<_, Vec<_>>::new(),
        |mut acc, t| {
            t.map(|t| {
                acc.entry(t.child_table.to_string()).or_default().push(t);
                acc
            })
        },
    )?;

    let mut table_pk_key_list = HashMap::new();
    let mut expected_schema_map = HashMap::new();

    for t in tables_from_schema.table_decls {
        let t = t?;
        let keys = t.primary_keys.as_ref().map(|keys| {
            keys.keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });
        table_pk_key_list.insert(t.view.table_name.to_string(), keys);
        expected_schema_map.insert(t.view.table_name.to_string(), t);
    }
    config.with_docs = DocConfig::NoDocComments;
    config.column_sorting = ColumnSorting::OrdinalPosition;

    // Parameter `sqlite_integer_primary_key_is_bigint` is only used for a SQLite connection
    match conn {
        #[cfg(feature = "postgres")]
        InferConnection::Pg(_) => config.sqlite_integer_primary_key_is_bigint = None,
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(_) => (),
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(_) => {
            config.sqlite_integer_primary_key_is_bigint = None;
        }
    }

    let mut schema_diff = Vec::new();
    let table_names = load_table_names(&mut conn, None)?;
    let tables_from_database =
        filter_table_names(&table_names, &config.filter, config.include_views);
    for (structure, table) in tables_from_database {
        tracing::info!(?table, "Diff for existing table");
        match structure {
            SupportedQueryRelationStructures::Table => {
                let columns = crate::infer_schema_internals::load_table_data(
                    &mut conn,
                    table.clone(),
                    &config,
                    structure,
                )?;
                if let Some(TableDecl { primary_keys, view }) =
                    expected_schema_map.remove(&table.sql_name.to_lowercase())
                {
                    tracing::info!(table = ?view.sql_name, "Table exists in schema.rs");
                    let mut primary_keys_in_db =
                        crate::infer_schema_internals::get_primary_keys(&mut conn, &table)?;
                    primary_keys_in_db.sort();
                    let mut primary_keys_in_schema = primary_keys
                        .map(|pk| pk.keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
                        .unwrap_or_else(|| vec!["id".into()]);
                    primary_keys_in_schema.sort();
                    if primary_keys_in_db != primary_keys_in_schema {
                        tracing::debug!(
                            ?primary_keys_in_schema,
                            ?primary_keys_in_db,
                            "Primary keys changed"
                        );
                        return Err(crate::errors::Error::UnsupportedFeature(
                            "Cannot change primary keys with --diff-schema yet".into(),
                        ));
                    }
                    schema_diff.push(update_columns(view, columns.column_data)?);
                } else {
                    tracing::info!("Table does not exist yet");
                    let foreign_keys = foreign_key_map
                        .get(&table.rust_name)
                        .cloned()
                        .unwrap_or_default();
                    if foreign_keys.iter().any(|fk| {
                        fk.foreign_key_columns.len() != 1 || fk.primary_key_columns.len() != 1
                    }) {
                        return Err(crate::errors::Error::UnsupportedFeature(
                            "Tables with composite foreign keys are not supported by --diff-schema"
                                .into(),
                        ));
                    }
                    schema_diff.push(SchemaDiff::DropTable {
                        table,
                        columns,
                        foreign_keys,
                    });
                }
            }
            SupportedQueryRelationStructures::View => {
                return Err(crate::errors::Error::UnsupportedFeature(
                    "Views are not supported by `--diff-schema`".into(),
                ));
            }
        }
    }

    schema_diff.extend(expected_schema_map.into_values().map(|t| {
        tracing::info!(table = ?t.view.sql_name, "Tables does not exist in database");
        let foreign_keys = expected_fk_map
            .remove(&t.view.table_name.to_string())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|j| {
                let referenced_table = table_pk_key_list.get(&j.parent_table.to_string())?;
                match referenced_table {
                    None => Some((j, "id".into())),
                    Some(pks) if pks.len() == 1 => Some((j, pks.first()?.to_string())),
                    Some(_) => None,
                }
            })
            .collect();
        SchemaDiff::CreateTable {
            to_create: t,
            foreign_keys,
        }
    }));

    let mut up_sql = String::new();
    let mut down_sql = String::new();

    for diff in schema_diff {
        let up = match conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(_) => {
                let mut qb = diesel::pg::PgQueryBuilder::default();
                diff.generate_up_sql(&mut qb, &config)?;
                qb.finish()
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(_) => {
                let mut qb = diesel::sqlite::SqliteQueryBuilder::default();
                diff.generate_up_sql(&mut qb, &config)?;
                qb.finish()
            }
            #[cfg(feature = "mysql")]
            InferConnection::Mysql(_) => {
                let mut qb = diesel::mysql::MysqlQueryBuilder::default();
                diff.generate_up_sql(&mut qb, &config)?;
                qb.finish()
            }
        };

        let down = match conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(_) => {
                let mut qb = diesel::pg::PgQueryBuilder::default();
                diff.generate_down_sql(&mut qb, &config)?;
                qb.finish()
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(_) => {
                let mut qb = diesel::sqlite::SqliteQueryBuilder::default();
                diff.generate_down_sql(&mut qb, &config)?;
                qb.finish()
            }
            #[cfg(feature = "mysql")]
            InferConnection::Mysql(_) => {
                let mut qb = diesel::mysql::MysqlQueryBuilder::default();
                diff.generate_down_sql(&mut qb, &config)?;
                qb.finish()
            }
        };
        up_sql += &up;
        up_sql += "\n";
        down_sql += &down;
        down_sql += "\n";
    }

    Ok((up_sql, down_sql))
}

fn update_columns(
    view: ViewDecl,
    columns: Vec<ColumnDefinition>,
) -> Result<SchemaDiff, crate::errors::Error> {
    let mut expected_column_map = view
        .column_defs
        .into_iter()
        .map(|c| (c.sql_name.to_lowercase(), c))
        .collect::<HashMap<_, _>>();

    let mut added_columns = Vec::new();
    let mut removed_columns = Vec::new();
    let mut changed_columns = Vec::new();

    for c in columns {
        if let Some(def) = expected_column_map.remove(&c.sql_name.to_lowercase()) {
            let tpe = ColumnType::for_column_def(&def)?;
            if !is_same_type(&c.ty, tpe) {
                tracing::info!(old = ?c, new = ?def.sql_name, "Column changed type");
                changed_columns.push((c, def));
            }
        } else {
            tracing::info!(column = ?c, "Column was removed");
            removed_columns.push(c);
        }
    }

    if !expected_column_map.is_empty() {
        let columns = expected_column_map
            .values()
            .map(|v| v.column_name.to_string())
            .collect::<Vec<_>>();
        tracing::info!(added = ?columns, "Added columns");
    }
    added_columns.extend(expected_column_map.into_values());

    Ok(SchemaDiff::ChangeTable {
        table: view.sql_name,
        added_columns,
        removed_columns,
        changed_columns,
    })
}

fn is_same_type(ty: &ColumnType, tpe: ColumnType) -> bool {
    if ty.is_array != tpe.is_array
        || ty.is_nullable != tpe.is_nullable
        || ty.is_unsigned != tpe.is_unsigned
        || ty.max_length != tpe.max_length
    {
        return false;
    }

    let mut is_same_schema = ty.schema == tpe.schema;
    if !is_same_schema
        && ((ty.schema.as_deref() == Some("pg_catalog") && tpe.schema.is_none())
            || (tpe.schema.as_deref() == Some("pg_catalog") && ty.schema.is_none()))
    {
        is_same_schema = true;
    }

    if ty.sql_name.to_lowercase() == tpe.sql_name.to_lowercase() && is_same_schema {
        return true;
    }
    if !is_same_schema {
        return false;
    }
    let compatible_types = compatible_type_list();

    if let Some(compatible) = compatible_types.get(&ty.sql_name.to_lowercase() as &str) {
        return compatible.contains(&((&tpe.sql_name.to_lowercase()) as &str));
    }
    if let Some(compatible) = compatible_types.get(&tpe.sql_name.to_lowercase() as &str) {
        return compatible.contains(&(&ty.sql_name.to_lowercase() as &str));
    }
    false
}

#[allow(clippy::enum_variant_names)]
enum SchemaDiff {
    DropTable {
        table: TableName,
        columns: TableData,
        foreign_keys: Vec<ForeignKeyConstraint>,
    },
    CreateTable {
        to_create: TableDecl,
        foreign_keys: Vec<(Joinable, String)>,
    },
    ChangeTable {
        table: String,
        added_columns: Vec<ColumnDef>,
        removed_columns: Vec<ColumnDefinition>,
        changed_columns: Vec<(ColumnDefinition, ColumnDef)>,
    },
}

impl SchemaDiff {
    fn generate_up_sql<DB>(
        &self,
        query_builder: &mut impl QueryBuilder<DB>,
        config: &PrintSchema,
    ) -> Result<(), crate::errors::Error>
    where
        DB: Backend,
    {
        match self {
            SchemaDiff::DropTable { table, .. } => {
                generate_drop_table(query_builder, &table.sql_name.to_lowercase())?;
            }
            SchemaDiff::CreateTable {
                to_create,
                foreign_keys,
            } => {
                let table = &to_create.view.sql_name.to_lowercase();
                let primary_keys = to_create
                    .primary_keys
                    .as_ref()
                    .map(|keys| keys.keys.iter().map(|k| k.to_string()).collect())
                    .unwrap_or_else(|| vec![String::from("id")]);
                let column_data = to_create
                    .view
                    .column_defs
                    .iter()
                    .map(|c| {
                        let ty = ColumnType::for_column_def(c)?;
                        Ok(ColumnDefinition {
                            sql_name: c.sql_name.to_lowercase(),
                            rust_name: c.sql_name.clone(),
                            ty,
                            comment: None,
                        })
                    })
                    .collect::<Result<Vec<_>, crate::errors::Error>>()?;

                let foreign_keys = foreign_keys
                    .iter()
                    .map(|(f, pk)| {
                        (
                            f.parent_table.to_string(),
                            f.ref_column.to_string(),
                            pk.clone(),
                        )
                    })
                    .collect::<Vec<_>>();

                let sqlite_integer_primary_key_is_bigint = config
                    .sqlite_integer_primary_key_is_bigint
                    .unwrap_or_default();
                collect_and_generate_record_types(query_builder, &column_data)?;
                generate_create_table(
                    query_builder,
                    table,
                    &column_data,
                    &primary_keys,
                    &foreign_keys,
                    sqlite_integer_primary_key_is_bigint,
                )?;
            }
            SchemaDiff::ChangeTable {
                table,
                added_columns,
                removed_columns,
                changed_columns,
            } => {
                for c in removed_columns
                    .iter()
                    .chain(changed_columns.iter().map(|(a, _)| a))
                {
                    generate_drop_column(query_builder, &table.to_lowercase(), &c.sql_name)?;
                    query_builder.push_sql("\n");
                }
                let for_record_types =
                    extract_record_types_from_changed_columns(added_columns, changed_columns)?;
                collect_and_generate_record_types(query_builder, &for_record_types)?;
                for c in added_columns
                    .iter()
                    .chain(changed_columns.iter().map(|(_, b)| b))
                {
                    generate_add_column(
                        query_builder,
                        &table.to_lowercase(),
                        &c.column_name.to_string().to_lowercase(),
                        &ColumnType::for_column_def(c)?,
                    )?;
                    query_builder.push_sql("\n");
                }
            }
        }
        Ok(())
    }

    fn generate_down_sql<DB>(
        &self,
        query_builder: &mut impl QueryBuilder<DB>,
        config: &PrintSchema,
    ) -> Result<(), crate::errors::Error>
    where
        DB: Backend,
    {
        match self {
            SchemaDiff::DropTable {
                table,
                columns,
                foreign_keys,
            } => {
                let fk = foreign_keys
                    .iter()
                    .map(|fk| {
                        (
                            fk.parent_table.rust_name.clone(),
                            fk.foreign_key_columns_rust[0].clone(),
                            fk.primary_key_columns[0].clone(),
                        )
                    })
                    .collect::<Vec<_>>();

                let sqlite_integer_primary_key_is_bigint = config
                    .sqlite_integer_primary_key_is_bigint
                    .unwrap_or_default();

                generate_create_table(
                    query_builder,
                    &table.sql_name.to_lowercase(),
                    &columns.column_data,
                    &columns.primary_key,
                    &fk,
                    sqlite_integer_primary_key_is_bigint,
                )?;
            }
            SchemaDiff::CreateTable { to_create, .. } => {
                generate_drop_table(query_builder, &to_create.view.sql_name.to_lowercase())?;
                let for_record_types = to_create
                    .view
                    .column_defs
                    .iter()
                    .map(|c| {
                        let ty = ColumnType::for_column_def(c)?;
                        Ok::<_, crate::errors::Error>(ColumnDefinition {
                            sql_name: c.sql_name.to_lowercase(),
                            rust_name: c.sql_name.clone(),
                            ty,
                            comment: None,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let record_types = collect_record_types(&for_record_types);
                drop_record_types(record_types, query_builder)?;
            }
            SchemaDiff::ChangeTable {
                table,
                added_columns,
                removed_columns,
                changed_columns,
            } => {
                // We don't need to check the `sqlite_integer_primary_key_is_bigint` parameter here
                // since `ÀLTER TABLE` queries cannot modify primary key columns in SQLite.
                // See https://www.sqlite.org/lang_altertable.html#alter_table_add_column for more information.
                for c in added_columns
                    .iter()
                    .chain(changed_columns.iter().map(|(_, b)| b))
                {
                    generate_drop_column(
                        query_builder,
                        &table.to_lowercase(),
                        &c.column_name.to_string().to_lowercase(),
                    )?;
                    query_builder.push_sql("\n");
                }
                let for_record_types =
                    extract_record_types_from_changed_columns(added_columns, changed_columns)?;
                let record_types = collect_record_types(&for_record_types);
                drop_record_types(record_types, query_builder)?;
                for c in removed_columns
                    .iter()
                    .chain(changed_columns.iter().map(|(a, _)| a))
                {
                    generate_add_column(
                        query_builder,
                        &table.to_lowercase(),
                        &c.sql_name.to_lowercase(),
                        &c.ty,
                    )?;
                    query_builder.push_sql("\n");
                }
            }
        }
        Ok(())
    }
}

fn extract_record_types_from_changed_columns(
    added_columns: &[ColumnDef],
    changed_columns: &[(ColumnDefinition, ColumnDef)],
) -> Result<Vec<ColumnDefinition>, crate::errors::Error> {
    let for_record_types = added_columns
        .iter()
        .map(|c| {
            let ty = ColumnType::for_column_def(c)?;
            Ok::<_, crate::errors::Error>(ColumnDefinition {
                sql_name: c.sql_name.to_lowercase(),
                rust_name: c.sql_name.clone(),
                ty,
                comment: None,
            })
        })
        .chain(changed_columns.iter().map(|(c, _)| Ok(c.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(for_record_types)
}

fn drop_record_types<DB>(
    record_types: Vec<(Cow<'_, str>, &[ColumnType])>,
    query_builder: &mut impl QueryBuilder<DB>,
) -> Result<(), crate::errors::Error>
where
    DB: Backend,
{
    if !record_types.is_empty() {
        query_builder.push_sql("\n\n");
    }
    for (column, _) in record_types.into_iter().rev() {
        drop_record_type(column, query_builder)?;
    }
    Ok(())
}

fn drop_record_type<DB>(
    column: Cow<'_, str>,
    query_builder: &mut impl QueryBuilder<DB>,
) -> Result<(), crate::errors::Error>
where
    DB: Backend,
{
    query_builder.push_sql("DROP TYPE IF EXISTS ");
    query_builder.push_identifier(&format!("{}_RECORD", column.to_uppercase()))?;
    query_builder.push_sql(";\n\n");
    Ok(())
}

fn collect_and_generate_record_types<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    column_data: &[ColumnDefinition],
) -> Result<(), crate::errors::Error>
where
    DB: Backend,
{
    let record_types = collect_record_types(column_data);
    for (column, record_types) in record_types {
        generate_record_types(query_builder, record_types, column)?;
    }
    Ok(())
}

// We need to return a boxed iterator here and not a impl Iterator as
// rustc otherwise returns a not very good error message that it cannot infer the type behind
// the impl
//
// I believe that's reasonable as rustc likely struggles with the recursive type otherwise
// Boxing just makes it much clearer what really happens here for the compiler
#[expect(
    clippy::type_complexity,
    reason = "Its an internally only type and it's not that complex"
)]
fn recursive_record_types<'a>(
    ty: &'a ColumnType,
    prefix: Cow<'a, str>,
) -> Box<dyn Iterator<Item = (Cow<'a, str>, Option<&'a [ColumnType]>)> + 'a> {
    let prefix_2 = prefix.clone();
    let iter = ty
        .record
        .as_deref()
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(move |(idx, c)| {
            recursive_record_types(c, Cow::Owned(format!("{prefix_2}_RECORD_FIELD_{idx}")))
        })
        .chain(
            ty.record
                .as_deref()
                .into_iter()
                .flatten()
                .enumerate()
                .map(move |(idx, c)| {
                    (
                        Cow::Owned(format!("{prefix}_RECORD_FIELD_{idx}")),
                        c.record.as_deref(),
                    )
                }),
        );
    Box::new(iter)
}

fn collect_record_types(column_data: &[ColumnDefinition]) -> Vec<(Cow<'_, str>, &[ColumnType])> {
    column_data
        .iter()
        .flat_map(|c| {
            recursive_record_types(&c.ty, Cow::Borrowed(c.sql_name.as_str())).chain(
                std::iter::once((Cow::Borrowed(c.sql_name.as_str()), c.ty.record.as_deref())),
            )
        })
        .filter_map(|(column_name, record)| Some((column_name, record?)))
        .collect::<Vec<_>>()
}

fn generate_record_types<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    record_types: &[ColumnType],
    column_name: Cow<'_, str>,
) -> QueryResult<()>
where
    DB: Backend,
{
    query_builder.push_sql("CREATE TYPE ");
    let record_type_name = format!("{}_RECORD", column_name.to_uppercase());
    if dbg!(record_type_name.len()) > 64 {
        return Err(diesel::result::Error::QueryBuilderError(
            format!(
                "Failed to construct a suitable name \
             for a record type for `{column_name}` due to PostgreSQL limiting \
             type names to 64 byte"
            )
            .into(),
        ));
    }
    query_builder.push_identifier(&record_type_name)?;
    query_builder.push_sql(" AS (\n");
    for (idx, record_type) in record_types.iter().enumerate() {
        query_builder.push_sql("\t");
        query_builder.push_identifier(&format!("FIELD_{idx}"))?;
        query_builder.push_sql(" ");
        generate_column_type_name(
            query_builder,
            record_type,
            &format!("{}_RECORD_FIELD_{idx}", column_name.to_uppercase()),
            true,
        )?;
        if idx != record_types.len() - 1 {
            query_builder.push_sql(",");
        }
        query_builder.push_sql("\n");
    }
    query_builder.push_sql(");\n\n");
    Ok(())
}

fn generate_add_column<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    table: &str,
    column: &str,
    ty: &ColumnType,
) -> QueryResult<()>
where
    DB: Backend,
{
    query_builder.push_sql("ALTER TABLE ");
    query_builder.push_identifier(table)?;
    query_builder.push_sql(" ADD COLUMN ");
    query_builder.push_identifier(column)?;
    generate_column_type_name(query_builder, ty, column, false)?;
    query_builder.push_sql(";");
    Ok(())
}

fn generate_drop_column<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    table: &str,
    column: &str,
) -> QueryResult<()>
where
    DB: Backend,
{
    query_builder.push_sql("ALTER TABLE ");
    query_builder.push_identifier(table)?;
    query_builder.push_sql(" DROP COLUMN ");
    query_builder.push_identifier(column)?;
    query_builder.push_sql(";");

    Ok(())
}

fn generate_create_table<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    table: &str,
    column_data: &[ColumnDefinition],
    primary_keys: &[String],
    foreign_keys: &[(String, String, String)],
    sqlite_integer_primary_key_is_bigint: bool,
) -> QueryResult<()>
where
    DB: Backend,
{
    query_builder.push_sql("CREATE TABLE ");
    query_builder.push_identifier(table)?;
    query_builder.push_sql("(\n");
    let mut first = true;
    let mut foreign_key_list = Vec::with_capacity(foreign_keys.len());
    for column in column_data {
        if first {
            first = false;
        } else {
            query_builder.push_sql(",\n");
        }
        query_builder.push_sql("\t");

        let is_only_primary_key =
            primary_keys.contains(&column.rust_name) && primary_keys.len() == 1;

        query_builder.push_identifier(&column.sql_name)?;

        // When the `sqlite_integer_primary_key_is_bigint` config parameter is used,
        // if a column is the only primary key and its type is `BigInt`,
        // we consider it equivalent to the `rowid` column in order to be compatible
        // with the `print-schema` command using the same config parameter.
        // See https://www.sqlite.org/lang_createtable.html#rowid for more information.
        if sqlite_integer_primary_key_is_bigint
            && is_only_primary_key
            && column.ty.sql_name.eq_ignore_ascii_case("BigInt")
        {
            let ty = ColumnType {
                rust_name: "Integer".into(),
                sql_name: "Integer".into(),
                ..column.ty.clone()
            };
            generate_column_type_name(query_builder, &ty, &column.sql_name, false)?;
        } else {
            generate_column_type_name(query_builder, &column.ty, &column.sql_name, false)?;
        }

        if is_only_primary_key {
            query_builder.push_sql(" PRIMARY KEY");
        }

        if let Some((table, _, pk)) = foreign_keys.iter().find(|(_, k, _)| k == &column.rust_name) {
            foreign_key_list.push((column, table, pk));
        }
    }
    if primary_keys.len() > 1 {
        query_builder.push_sql(",\n");
        query_builder.push_sql("\tPRIMARY KEY(");
        for (idx, key) in primary_keys.iter().enumerate() {
            query_builder.push_identifier(key)?;
            if idx != primary_keys.len() - 1 {
                query_builder.push_sql(", ");
            }
        }
        query_builder.push_sql(")");
    }
    // MySQL parses but ignores “inline REFERENCES specifications”
    // (as defined in the SQL standard)
    // where the references are defined as part of the column specification.
    // MySQL accepts REFERENCES clauses only when specified as
    // part of a separate FOREIGN KEY specification.
    //
    // https://dev.mysql.com/doc/refman/8.0/en/ansi-diff-foreign-keys.html

    for (column, table, pk) in foreign_key_list {
        query_builder.push_sql(",\n\t");
        query_builder.push_sql("FOREIGN KEY (");
        query_builder.push_identifier(&column.sql_name)?;
        query_builder.push_sql(") REFERENCES ");
        query_builder.push_identifier(table)?;
        query_builder.push_sql("(");
        query_builder.push_identifier(pk)?;
        query_builder.push_sql(")");
    }
    query_builder.push_sql("\n);");

    query_builder.push_sql("\n");

    Ok(())
}

fn generate_column_type_name<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    ty: &ColumnType,
    column_name: &str,
    for_record: bool,
) -> QueryResult<()>
where
    DB: Backend,
{
    // TODO: handle schema
    if ty.record.is_some() {
        query_builder.push_sql(" ");
        // need to quote the type name here as we quote it during creating which creates an upper case only type
        query_builder.push_identifier(&format!("{}_RECORD", column_name.to_uppercase()))?;
    } else {
        query_builder.push_sql(&format!(" {}", ty.sql_name.to_uppercase()));
    }
    if ty.is_array {
        query_builder.push_sql("[]");
    }
    if let Some(max_length) = ty.max_length {
        query_builder.push_sql(&format!("({max_length})"));
    }
    if !for_record {
        if !ty.is_nullable {
            query_builder.push_sql(" NOT NULL");
        }
        if ty.is_unsigned {
            query_builder.push_sql(" UNSIGNED");
        }
    }
    Ok(())
}

fn generate_drop_table<DB>(
    query_builder: &mut impl QueryBuilder<DB>,
    table: &str,
) -> QueryResult<()>
where
    DB: Backend,
{
    // TODO: handle schema?
    query_builder.push_sql("DROP TABLE IF EXISTS ");
    query_builder.push_identifier(table)?;
    query_builder.push_sql(";");
    Ok(())
}

struct Joinable {
    parent_table: syn::Ident,
    child_table: syn::Ident,
    ref_column: syn::Ident,
}

impl syn::parse::Parse for Joinable {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let child_table = input.parse()?;
        let _arrow: syn::Token![->] = input.parse()?;
        let parent_table = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let ref_column = content.parse()?;
        Ok(Self {
            child_table,
            parent_table,
            ref_column,
        })
    }
}

#[derive(Default)]
struct SchemaCollector {
    table_decls: Vec<Result<TableDecl, syn::Error>>,
    joinable: Vec<Result<Joinable, syn::Error>>,
}

impl<'ast> syn::visit::Visit<'ast> for SchemaCollector {
    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        let last_segment = i.path.segments.last();
        if last_segment.map(|s| s.ident == "table").unwrap_or(false) {
            self.table_decls.push(i.parse_body());
        } else if last_segment.map(|s| s.ident == "joinable").unwrap_or(false) {
            self.joinable.push(i.parse_body());
        }
        syn::visit::visit_macro(self, i)
    }
}
