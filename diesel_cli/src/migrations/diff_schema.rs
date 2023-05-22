use clap::ArgMatches;
use diesel::backend::Backend;
use diesel::query_builder::QueryBuilder;
use diesel::QueryResult;
use diesel_table_macro_syntax::{ColumnDef, TableDecl};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use syn::visit::Visit;

use crate::config::Config;
use crate::database::InferConnection;
use crate::infer_schema_internals::{
    ColumnDefinition, ColumnType, ForeignKeyConstraint, TableData, TableName,
};
use crate::print_schema::DocConfig;

fn compatible_type_list() -> HashMap<&'static str, Vec<&'static str>> {
    let mut map = HashMap::new();
    map.insert("integer", vec!["int4"]);
    map.insert("bigint", vec!["int8"]);
    map.insert("smallint", vec!["int2"]);
    map.insert("text", vec!["varchar"]);
    map
}

pub fn generate_sql_based_on_diff_schema(
    _config: Config,
    matches: &ArgMatches,
    schema_file_path: &Path,
) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    let project_root = crate::find_project_root()?;

    let schema_path = project_root.join(schema_file_path);
    let mut schema_file = File::open(schema_path)?;
    let mut content = String::new();
    schema_file.read_to_string(&mut content)?;
    let syn_file = syn::parse_file(&content)?;

    let mut tables_from_schema = SchemaCollector::default();

    tables_from_schema.visit_file(&syn_file);
    let mut conn = InferConnection::from_matches(matches);
    let tables_from_database = crate::infer_schema_internals::load_table_names(&mut conn, None)?;
    let foreign_keys =
        crate::infer_schema_internals::load_foreign_key_constraints(&mut conn, None)?;
    let foreign_key_map = foreign_keys.into_iter().fold(HashMap::new(), |mut acc, t| {
        acc.entry(t.child_table.rust_name.clone())
            .or_insert_with(Vec::new)
            .push(t);
        acc
    });

    let mut expected_fk_map =
        tables_from_schema
            .joinable
            .into_iter()
            .try_fold(HashMap::new(), |mut acc, t| {
                t.map(|t| {
                    acc.entry(t.child_table.to_string())
                        .or_insert_with(Vec::new)
                        .push(t);
                    acc
                })
            })?;

    let table_pk_key_list = tables_from_schema
        .table_decls
        .iter()
        .map(|t| {
            let t = t.as_ref().unwrap();
            Ok((
                t.table_name.to_string(),
                t.primary_keys.as_ref().map(|keys| {
                    keys.keys
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                }),
            ))
        })
        .collect::<Result<HashMap<_, _>, &syn::Error>>()?;

    let mut expected_schema_map = tables_from_schema
        .table_decls
        .into_iter()
        .map(|t| {
            let t = t?;
            Ok((t.table_name.to_string().to_lowercase(), t))
        })
        .collect::<Result<HashMap<_, _>, syn::Error>>()?;

    let mut schema_diff = Vec::new();

    for table in tables_from_database {
        let columns = crate::infer_schema_internals::load_table_data(
            &mut conn,
            table.clone(),
            &crate::print_schema::ColumnSorting::OrdinalPosition,
            DocConfig::NoDocComments,
        )?;
        if let Some(t) = expected_schema_map.remove(&table.sql_name.to_lowercase()) {
            let mut primary_keys_in_db =
                crate::infer_schema_internals::get_primary_keys(&mut conn, &table)?;
            primary_keys_in_db.sort();
            let mut primary_keys_in_schema = t
                .primary_keys
                .map(|pk| pk.keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
                .unwrap_or_else(|| vec!["id".into()]);
            primary_keys_in_schema.sort();
            if primary_keys_in_db != primary_keys_in_schema {
                return Err("Cannot change primary keys with --diff-schema yet".into());
            }

            let mut expected_column_map = t
                .column_defs
                .into_iter()
                .map(|c| (c.sql_name.to_lowercase(), c))
                .collect::<HashMap<_, _>>();

            let mut added_columns = Vec::new();
            let mut removed_columns = Vec::new();
            let mut changed_columns = Vec::new();

            for c in columns.column_data {
                if let Some(def) = expected_column_map.remove(&c.sql_name.to_lowercase()) {
                    let tpe = ColumnType::for_column_def(&def)?;
                    if !is_same_type(&c.ty, tpe) {
                        changed_columns.push((c, def));
                    }
                } else {
                    removed_columns.push(c);
                }
            }

            added_columns.extend(expected_column_map.into_values());

            schema_diff.push(SchemaDiff::ChangeTable {
                table: t.sql_name,
                added_columns,
                removed_columns,
                changed_columns,
            });
        } else {
            let foreign_keys = foreign_key_map
                .get(&table.rust_name)
                .cloned()
                .unwrap_or_default();
            if foreign_keys
                .iter()
                .any(|fk| fk.foreign_key_columns.len() != 1 || fk.primary_key_columns.len() != 1)
            {
                return Err(
                    "Tables with composite foreign keys are not supported by --diff-schema".into(),
                );
            }
            schema_diff.push(SchemaDiff::DropTable {
                table,
                columns,
                foreign_keys,
            });
        }
    }

    schema_diff.extend(expected_schema_map.into_values().map(|t| {
        let foreign_keys = expected_fk_map
            .remove(&t.table_name.to_string())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|j| {
                let referenced_table = table_pk_key_list.get(&t.table_name.to_string())?;
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
                diff.generate_up_sql(&mut qb)?;
                qb.finish()
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(_) => {
                let mut qb = diesel::sqlite::SqliteQueryBuilder::default();
                diff.generate_up_sql(&mut qb)?;
                qb.finish()
            }
            #[cfg(feature = "mysql")]
            InferConnection::Mysql(_) => {
                let mut qb = diesel::mysql::MysqlQueryBuilder::default();
                diff.generate_up_sql(&mut qb)?;
                qb.finish()
            }
        };

        let down = match conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(_) => {
                let mut qb = diesel::pg::PgQueryBuilder::default();
                diff.generate_down_sql(&mut qb)?;
                qb.finish()
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(_) => {
                let mut qb = diesel::sqlite::SqliteQueryBuilder::default();
                diff.generate_down_sql(&mut qb)?;
                qb.finish()
            }
            #[cfg(feature = "mysql")]
            InferConnection::Mysql(_) => {
                let mut qb = diesel::mysql::MysqlQueryBuilder::default();
                diff.generate_down_sql(&mut qb)?;
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
    fn generate_up_sql<DB>(&self, query_builder: &mut impl QueryBuilder<DB>) -> QueryResult<()>
    where
        DB: Backend,
    {
        match self {
            SchemaDiff::DropTable { table, .. } => {
                generate_drop_table(query_builder, &table.sql_name.to_lowercase())?;
            }
            SchemaDiff::CreateTable {
                to_create: to_ceate,
                foreign_keys,
            } => {
                let table = &to_ceate.sql_name.to_lowercase();
                let primary_keys = to_ceate
                    .primary_keys
                    .as_ref()
                    .map(|keys| keys.keys.iter().map(|k| k.to_string()).collect())
                    .unwrap_or_else(|| vec![String::from("id")]);
                let column_data = to_ceate
                    .column_defs
                    .iter()
                    .map(|c| {
                        let ty = ColumnType::for_column_def(c)
                            .map_err(diesel::result::Error::QueryBuilderError)?;
                        Ok(ColumnDefinition {
                            sql_name: c.sql_name.to_lowercase(),
                            rust_name: c.sql_name.clone(),
                            ty,
                            comment: None,
                        })
                    })
                    .collect::<QueryResult<Vec<_>>>()?;
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
                generate_create_table(
                    query_builder,
                    table,
                    &column_data,
                    &primary_keys,
                    &foreign_keys,
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
                for c in added_columns
                    .iter()
                    .chain(changed_columns.iter().map(|(_, b)| b))
                {
                    generate_add_column(
                        query_builder,
                        &table.to_lowercase(),
                        &c.column_name.to_string().to_lowercase(),
                        &ColumnType::for_column_def(c)
                            .map_err(diesel::result::Error::QueryBuilderError)?,
                    )?;
                    query_builder.push_sql("\n");
                }
            }
        }
        Ok(())
    }

    fn generate_down_sql<DB>(&self, query_builder: &mut impl QueryBuilder<DB>) -> QueryResult<()>
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

                generate_create_table(
                    query_builder,
                    &table.sql_name.to_lowercase(),
                    &columns.column_data,
                    &columns.primary_key,
                    &fk,
                )?;
            }
            SchemaDiff::CreateTable {
                to_create: to_ceate,
                ..
            } => {
                generate_drop_table(query_builder, &to_ceate.sql_name.to_lowercase())?;
            }
            SchemaDiff::ChangeTable {
                table,
                added_columns,
                removed_columns,
                changed_columns,
            } => {
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
    generate_column_type_name(query_builder, ty);
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
        query_builder.push_identifier(&column.sql_name)?;
        generate_column_type_name(query_builder, &column.ty);
        if primary_keys.contains(&column.rust_name) && primary_keys.len() == 1 {
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

fn generate_column_type_name<DB>(query_builder: &mut impl QueryBuilder<DB>, ty: &ColumnType)
where
    DB: Backend,
{
    // TODO: handle schema
    query_builder.push_sql(&format!(" {}", ty.sql_name.to_uppercase()));
    if let Some(max_length) = ty.max_length {
        query_builder.push_sql(&format!("({max_length})"));
    }
    if !ty.is_nullable {
        query_builder.push_sql(" NOT NULL");
    }
    if ty.is_unsigned {
        query_builder.push_sql(" UNSIGNED");
    }
    if ty.is_array {
        query_builder.push_sql("[]");
    }
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
