use diesel::QueryResult;
use diesel::backend::Backend;
use diesel::query_builder::{QueryBuilder, QueryFragment};
use diesel_table_macro_syntax::{ColumnDef, TableDecl, ViewDecl};
use enum_type_query_fragments::{
    AddEnumVariants, CreateEnumType, DropEnumType, EnumType, MigrateEnumData,
};
use schema_parsing::{EnumVariant, SqlTypeInfo};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use syn::visit::Visit;

use crate::config::PrintSchema;
use crate::database::InferConnection;
use crate::infer_schema_internals::{
    self, ColumnDefinition, ColumnType, ForeignKeyConstraint, QueryRelationData,
    SupportedQueryRelationStructures, TableData, TableName, filter_table_names, load_table_names,
};
use crate::print_schema::{ColumnSorting, DocConfig};

mod enum_type_query_fragments;
mod schema_parsing;

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
    config.set_filter(&table_name, &only_tables, &except_tables)?;

    let project_root = crate::find_project_root()?;

    let schema_path = project_root.join(schema_file_path);
    let content = std::fs::read_to_string(&schema_path)
        .map_err(|e| crate::errors::Error::IoError(e, Some(schema_path.clone())))?;

    let syn_file = syn::parse_file(&content)?;

    let mut rust_side_schema = schema_parsing::SchemaCollector::default();

    rust_side_schema.visit_file(&syn_file);
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

    let mut expected_fk_map =
        rust_side_schema
            .joinable
            .iter()
            .try_fold(HashMap::<_, Vec<_>>::new(), |mut acc, t| {
                t.clone().map(|t| {
                    acc.entry(t.child_table.to_string()).or_default().push(t);
                    acc
                })
            })?;

    let mut table_pk_key_list = HashMap::new();
    let mut expected_schema_map = HashMap::new();

    for t in rust_side_schema.table_decls.clone() {
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
    let mut table_data = Vec::new();
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
                table_data.push(QueryRelationData::Table(columns.clone()));
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
                    schema_diff.push(update_columns(
                        view,
                        columns.column_data,
                        &rust_side_schema.enum_sql_types,
                    )?);
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

    let mut schema_diff_down = schema_diff.clone();

    let mut enum_sql_types = Vec::new();
    if config.generate_missing_sql_type_definitions() && config.generate_rust_enum_definitions() {
        let custom_type_infos =
            crate::print_schema::load_custom_types(&mut conn, &table_data, &config)?;
        let mut sql_types = rust_side_schema.enum_sql_types.clone();

        for (ty, tab, col) in custom_type_infos
            .custom_type_list
            .iter()
            .zip(&table_data)
            .flat_map(|(c, t)| c.iter().zip(t.columns()).map(move |(ct, c)| (ct, t, c)))
            .filter_map(|(ty, tab, col)| Some((ty.as_ref()?, tab, col)))
        {
            if let Some(t) = custom_type_infos
                .enum_variant_list
                .get(&(ty.sql_name.clone(), ty.schema.clone()))
                .cloned()
                .or({
                    #[cfg(feature = "mysql")]
                    {
                        crate::infer_schema_internals::mysql::get_enum_variants(ty)
                    }
                    #[cfg(not(feature = "mysql"))]
                    None
                })
            {
                if let Some(rust_side_type_idx) = sql_types
                    .iter()
                    .position(|t| t.is_type(ty, &rust_side_schema, col, tab))
                    && let Some(rust_variants) = rust_side_schema.enums.iter().find(|t| {
                        t.sql_type
                            .path
                            .segments
                            .last()
                            .map(|s| s.ident == sql_types[rust_side_type_idx].rust_name)
                            .unwrap_or_default()
                    })
                {
                    let sql_type_info = sql_types.remove(rust_side_type_idx);
                    enum_sql_types.push((sql_type_info.clone(), rust_variants.clone()));
                    let rust_variant_set = rust_variants
                        .variants
                        .iter()
                        .map(|v| &v.sql_name)
                        .collect::<BTreeSet<_>>();
                    let sql_variant_set = t.iter().map(|v| &v.sql_name).collect::<BTreeSet<_>>();
                    let added_variants = rust_variant_set
                        .difference(&sql_variant_set)
                        .copied()
                        .cloned()
                        .collect::<Vec<_>>();
                    let removed_variants = sql_variant_set
                        .difference(&rust_variant_set)
                        .copied()
                        .cloned()
                        .collect::<Vec<_>>();
                    let mut affected_tables = table_data
                        .iter()
                        .flat_map(|t| t.columns().iter().map(move |c| (c, t)))
                        .filter(|(c, _t)| sql_type_info.rust_name == c.ty.rust_name)
                        .map(|(c, t)| (c.clone(), t.clone()))
                        .collect::<Vec<_>>();
                    if affected_tables.is_empty() {
                        affected_tables.push((col.clone(), tab.clone()));
                    }

                    if !added_variants.is_empty() || !removed_variants.is_empty() {
                        if removed_variants.is_empty() {
                            schema_diff.push(SchemaDiff::AddEnumVariant {
                                added_variants: added_variants.clone(),
                                all_variants: rust_variants
                                    .variants
                                    .iter()
                                    .map(|v| v.sql_name.clone())
                                    .collect(),
                                type_info: sql_type_info.clone(),
                                column_info: Some((tab.table_name().sql_name.clone(), col.clone())),
                            });
                        } else {
                            schema_diff.push(SchemaDiff::MigrateEnumData {
                                affected_tables: affected_tables.clone(),
                                type_info: sql_type_info.clone(),
                                infos: rust_variants.clone(),
                            });
                        }
                        if added_variants.is_empty() {
                            schema_diff_down.push(SchemaDiff::AddEnumVariant {
                                added_variants: removed_variants,
                                type_info: sql_type_info.clone(),
                                all_variants: t.iter().map(|v| v.sql_name.clone()).collect(),
                                column_info: Some((tab.table_name().sql_name.clone(), col.clone())),
                            });
                        } else {
                            schema_diff_down.push(SchemaDiff::MigrateEnumData {
                                affected_tables,
                                type_info: sql_type_info.clone(),
                                infos: schema_parsing::EnumInfos {
                                    variants: t
                                        .iter()
                                        .map(|v| EnumVariant {
                                            sql_name: v.sql_name.clone(),
                                        })
                                        .collect(),
                                    sql_type: syn::parse_str(&ty.rust_name)?,
                                },
                            });
                        }
                    }
                } else {
                    let drop_enum = SchemaDiff::DropEnum {
                        old_variants: t.clone(),
                        old_type: ty.clone(),
                    };
                    schema_diff.push(drop_enum.clone());
                    schema_diff_down.push(drop_enum);
                    enum_sql_types.push((
                        schema_parsing::SqlTypeInfo::from_column_type(ty)?,
                        schema_parsing::EnumInfos {
                            variants: t
                                .into_iter()
                                .map(|v| schema_parsing::EnumVariant {
                                    sql_name: v.sql_name.clone(),
                                })
                                .collect(),
                            sql_type: syn::parse_str(&ty.rust_name)?,
                        },
                    ));
                }
            }
        }
        for tpe in sql_types {
            if let Some(rust_variants) = rust_side_schema.enums.iter().find(|t| {
                t.sql_type
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == tpe.rust_name)
                    .unwrap_or_default()
            }) {
                enum_sql_types.push((tpe.clone(), rust_variants.clone()));
                let add_enum = SchemaDiff::AddEnum {
                    infos: rust_variants.clone(),
                    type_info: tpe,
                };
                schema_diff.push(add_enum.clone());
                schema_diff_down.push(add_enum);
            } else {
                tracing::debug!(?tpe, "Enum sql type not found");
            }
        }
    }

    let mut up_sql = String::new();

    // sort so that types come before tables
    schema_diff.sort_by(SchemaDiff::cmp);

    for diff in schema_diff {
        let up = match conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(_) => {
                let mut qb = diesel::pg::PgQueryBuilder::default();
                diff.generate_up_sql(&mut qb, &config, &enum_sql_types, &diesel::pg::Pg)?;
                qb.finish()
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(_) => {
                let mut qb = diesel::sqlite::SqliteQueryBuilder::default();
                diff.generate_up_sql(&mut qb, &config, &enum_sql_types, &diesel::sqlite::Sqlite)?;
                qb.finish()
            }
            #[cfg(feature = "mysql")]
            InferConnection::Mysql(_) => {
                let mut qb = diesel::mysql::MysqlQueryBuilder::default();
                diff.generate_up_sql(&mut qb, &config, &enum_sql_types, &diesel::mysql::Mysql)?;
                qb.finish()
            }
        };
        if !up.is_empty() {
            up_sql += &up;
            up_sql += "\n";
        }
    }

    // sort so that tables come before types
    schema_diff_down.sort_by(|a, b| SchemaDiff::cmp(a, b).reverse());

    let mut down_sql = String::new();
    for diff in schema_diff_down {
        let down = match conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(_) => {
                let mut qb = diesel::pg::PgQueryBuilder::default();
                diff.generate_down_sql(&mut qb, &config, &enum_sql_types, &diesel::pg::Pg)?;
                qb.finish()
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(_) => {
                let mut qb = diesel::sqlite::SqliteQueryBuilder::default();
                diff.generate_down_sql(&mut qb, &config, &enum_sql_types, &diesel::sqlite::Sqlite)?;
                qb.finish()
            }
            #[cfg(feature = "mysql")]
            InferConnection::Mysql(_) => {
                let mut qb = diesel::mysql::MysqlQueryBuilder::default();
                diff.generate_down_sql(&mut qb, &config, &enum_sql_types, &diesel::mysql::Mysql)?;
                qb.finish()
            }
        };
        if !down.is_empty() {
            down_sql += &down;
            down_sql += "\n";
        }
    }

    Ok((up_sql, down_sql))
}

fn update_columns(
    view: ViewDecl,
    columns: Vec<ColumnDefinition>,
    rust_enum_sql_types: &[SqlTypeInfo],
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
            if !is_same_type(&c.ty, tpe, rust_enum_sql_types) {
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

fn is_same_type(ty: &ColumnType, tpe: ColumnType, rust_enum_sql_types: &[SqlTypeInfo]) -> bool {
    #[cfg(feature = "mysql")]
    {
        if crate::infer_schema_internals::mysql::get_enum_variants(ty).is_some()
            && rust_enum_sql_types
                .iter()
                .any(|e| e.rust_name == tpe.rust_name)
        {
            return true;
        }
    }
    #[cfg(not(feature = "mysql"))]
    let _ = rust_enum_sql_types;
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
    ty.rust_name == tpe.rust_name
}

#[derive(Clone)]
#[allow(clippy::enum_variant_names)]
enum SchemaDiff {
    DropTable {
        table: TableName,
        columns: TableData,
        foreign_keys: Vec<ForeignKeyConstraint>,
    },
    CreateTable {
        to_create: TableDecl,
        foreign_keys: Vec<(schema_parsing::Joinable, String)>,
    },
    ChangeTable {
        table: String,
        added_columns: Vec<ColumnDef>,
        removed_columns: Vec<ColumnDefinition>,
        changed_columns: Vec<(ColumnDefinition, ColumnDef)>,
    },
    DropEnum {
        old_variants: Vec<infer_schema_internals::EnumVariant>,
        old_type: ColumnType,
    },
    AddEnum {
        infos: schema_parsing::EnumInfos,
        type_info: schema_parsing::SqlTypeInfo,
    },
    AddEnumVariant {
        added_variants: Vec<String>,
        all_variants: Vec<String>,
        type_info: schema_parsing::SqlTypeInfo,
        column_info: Option<(String, ColumnDefinition)>,
    },
    MigrateEnumData {
        affected_tables: Vec<(ColumnDefinition, QueryRelationData)>,
        type_info: schema_parsing::SqlTypeInfo,
        infos: schema_parsing::EnumInfos,
    },
}

impl SchemaDiff {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::AddEnum { .. }, _) => std::cmp::Ordering::Less,
            (Self::DropEnum { .. }, _) => std::cmp::Ordering::Greater,
            (_, Self::AddEnum { .. }) => std::cmp::Ordering::Greater,
            (_, Self::DropEnum { .. }) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn generate_up_sql<DB>(
        &self,
        query_builder: &mut DB::QueryBuilder,
        config: &PrintSchema,
        enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
        db: &DB,
    ) -> Result<(), crate::errors::Error>
    where
        DB: Backend,
        for<'a> CreateEnumType<'a>: QueryFragment<DB>,
        for<'a> EnumType<'a>: QueryFragment<DB>,
        for<'a> DropEnumType<'a>: QueryFragment<DB>,
        for<'a> AddEnumVariants<'a>: QueryFragment<DB>,
        for<'a> MigrateEnumData<'a>: QueryFragment<DB>,
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
                collect_and_generate_record_types(
                    query_builder,
                    &column_data,
                    enum_sql_types,
                    db,
                    table,
                )?;
                generate_create_table(
                    query_builder,
                    table,
                    &column_data,
                    &primary_keys,
                    &foreign_keys,
                    sqlite_integer_primary_key_is_bigint,
                    enum_sql_types,
                    db,
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
                collect_and_generate_record_types(
                    query_builder,
                    &for_record_types,
                    enum_sql_types,
                    db,
                    table,
                )?;
                for c in added_columns
                    .iter()
                    .chain(changed_columns.iter().map(|(_, b)| b))
                {
                    generate_add_column(
                        query_builder,
                        &table.to_lowercase(),
                        &c.column_name.to_string().to_lowercase(),
                        &ColumnType::for_column_def(c)?,
                        enum_sql_types,
                        db,
                    )?;
                    query_builder.push_sql("\n");
                }
            }
            Self::DropEnum { old_type, .. } => {
                DropEnumType {
                    tpe: &SqlTypeInfo::from_column_type(old_type)?,
                }
                .to_sql(query_builder, db)?;
            }
            Self::AddEnum { infos, type_info } => {
                let create_type = CreateEnumType {
                    tpe: type_info,
                    variants: &infos.variants,
                };
                create_type.to_sql(query_builder, db)?;
            }
            Self::AddEnumVariant {
                added_variants,
                all_variants,
                type_info,
                column_info,
            } => {
                AddEnumVariants {
                    added_variants,
                    all_variants,
                    tpe: type_info,
                    column_info: column_info.as_ref().map(|v| (v.0.as_str(), &v.1)),
                }
                .to_sql(query_builder, db)?;
            }
            Self::MigrateEnumData {
                affected_tables,
                type_info,
                infos,
            } => {
                MigrateEnumData::new(affected_tables, type_info, enum_sql_types, infos)
                    .to_sql(query_builder, db)?;
            }
        }
        Ok(())
    }

    fn generate_down_sql<DB>(
        &self,
        query_builder: &mut DB::QueryBuilder,
        config: &PrintSchema,
        enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
        db: &DB,
    ) -> Result<(), crate::errors::Error>
    where
        DB: Backend,
        for<'a> EnumType<'a>: QueryFragment<DB>,
        for<'a> DropEnumType<'a>: QueryFragment<DB>,
        for<'a> CreateEnumType<'a>: QueryFragment<DB>,
        for<'a> MigrateEnumData<'a>: QueryFragment<DB>,
        for<'a> AddEnumVariants<'a>: QueryFragment<DB>,
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
                    enum_sql_types,
                    db,
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
                        enum_sql_types,
                        db,
                    )?;
                    query_builder.push_sql("\n");
                }
            }
            Self::DropEnum {
                old_variants,
                old_type,
            } => {
                CreateEnumType {
                    tpe: &SqlTypeInfo::from_column_type(old_type)?,
                    variants: &old_variants
                        .iter()
                        .map(|v| EnumVariant {
                            sql_name: v.sql_name.clone(),
                        })
                        .collect::<Vec<_>>(),
                }
                .to_sql(query_builder, db)?;
            }
            Self::AddEnum { type_info, .. } => {
                DropEnumType { tpe: type_info }.to_sql(query_builder, db)?;
            }
            Self::AddEnumVariant {
                added_variants,
                type_info,
                all_variants,
                column_info,
            } => {
                AddEnumVariants {
                    added_variants,
                    tpe: type_info,
                    all_variants,
                    column_info: column_info.as_ref().map(|(t, c)| (t.as_str(), c)),
                }
                .to_sql(query_builder, db)?;
            }
            Self::MigrateEnumData {
                affected_tables,
                type_info,
                infos,
            } => {
                MigrateEnumData::new(affected_tables, type_info, enum_sql_types, infos)
                    .to_sql(query_builder, db)?;
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
    query_builder: &mut DB::QueryBuilder,
    column_data: &[ColumnDefinition],
    enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
    db: &DB,
    table_name: &str,
) -> Result<(), crate::errors::Error>
where
    DB: Backend,
    for<'a> EnumType<'a>: QueryFragment<DB>,
{
    let record_types = collect_record_types(column_data);
    for (column, record_types) in record_types {
        generate_record_types(
            query_builder,
            record_types,
            column,
            enum_sql_types,
            db,
            table_name,
        )?;
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
    query_builder: &mut DB::QueryBuilder,
    record_types: &[ColumnType],
    column_name: Cow<'_, str>,
    enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
    db: &DB,
    table_name: &str,
) -> QueryResult<()>
where
    DB: Backend,
    for<'a> EnumType<'a>: QueryFragment<DB>,
{
    query_builder.push_sql("CREATE TYPE ");
    let record_type_name = format!("{}_RECORD", column_name.to_uppercase());
    if record_type_name.len() > 64 {
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
            enum_sql_types,
            db,
            table_name,
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
    query_builder: &mut DB::QueryBuilder,
    table: &str,
    column: &str,
    ty: &ColumnType,
    enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
    db: &DB,
) -> QueryResult<()>
where
    DB: Backend,
    for<'a> EnumType<'a>: QueryFragment<DB>,
{
    query_builder.push_sql("ALTER TABLE ");
    query_builder.push_identifier(table)?;
    query_builder.push_sql(" ADD COLUMN ");
    query_builder.push_identifier(column)?;
    generate_column_type_name(query_builder, ty, column, false, enum_sql_types, db, table)?;
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

#[allow(clippy::too_many_arguments)]
fn generate_create_table<DB>(
    query_builder: &mut DB::QueryBuilder,
    table: &str,
    column_data: &[ColumnDefinition],
    primary_keys: &[String],
    foreign_keys: &[(String, String, String)],
    sqlite_integer_primary_key_is_bigint: bool,
    enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
    db: &DB,
) -> QueryResult<()>
where
    DB: Backend,
    for<'a> EnumType<'a>: QueryFragment<DB>,
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
            generate_column_type_name(
                query_builder,
                &ty,
                &column.sql_name,
                false,
                enum_sql_types,
                db,
                table,
            )?;
        } else {
            generate_column_type_name(
                query_builder,
                &column.ty,
                &column.sql_name,
                false,
                enum_sql_types,
                db,
                table,
            )?;
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

struct ColumnTypeName<'a> {
    ty: Cow<'a, ColumnType>,
    column_name: &'a str,
    for_record: bool,
    enum_type: Option<EnumType<'a>>,
}

impl<'a> ColumnTypeName<'a> {
    fn new(
        ty: &'a ColumnType,
        column_name: &'a str,
        for_record: bool,
        enum_sql_types: &'a [(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
        table_name: &str,
    ) -> Self {
        #[cfg(feature = "mysql")]
        let ty = if crate::infer_schema_internals::mysql::get_enum_variants(ty).is_some() {
            let mut ty = ty.clone();
            ty.rust_name = crate::print_schema::mysql_enum_name(table_name, column_name, &ty);
            ty.max_length = None;
            Cow::Owned(ty)
        } else {
            Cow::Borrowed(ty)
        };
        #[cfg(not(feature = "mysql"))]
        let ty = {
            let _ = table_name;
            Cow::Borrowed(ty)
        };
        let enum_type = if let Some((enum_type, enum_infos)) = enum_sql_types
            .iter()
            .find(|(t, _)| t.rust_name == ty.rust_name)
        {
            Some(EnumType {
                tpe: enum_type,
                variants: &enum_infos.variants,
            })
        } else {
            None
        };

        Self {
            ty,
            column_name,
            for_record,
            enum_type,
        }
    }
}

impl<DB> QueryFragment<DB> for ColumnTypeName<'_>
where
    DB: Backend,
    for<'a> EnumType<'a>: QueryFragment<DB>,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> QueryResult<()> {
        if let Some(enum_type) = self.enum_type.as_ref() {
            pass.push_sql(" ");
            enum_type.walk_ast(pass.reborrow())?;
        } else if self.ty.record.is_some() {
            // TODO: handle schema
            pass.push_sql(" ");
            // need to quote the type name here as we quote it during creating which creates an upper case only type
            pass.push_identifier(&format!("{}_RECORD", self.column_name.to_uppercase()))?;
        } else {
            // TODO: handle schema
            pass.push_sql(&format!(" {}", self.ty.sql_name.to_uppercase()));
        }
        if self.ty.is_array {
            pass.push_sql("[]");
        }
        if let Some(max_length) = self.ty.max_length {
            pass.push_sql(&format!("({max_length})"));
        }
        if !self.for_record {
            if !self.ty.is_nullable {
                pass.push_sql(" NOT NULL");
            }
            if self.ty.is_unsigned {
                pass.push_sql(" UNSIGNED");
            }
        }
        Ok(())
    }
}

fn generate_column_type_name<DB>(
    query_builder: &mut DB::QueryBuilder,
    ty: &ColumnType,
    column_name: &str,
    for_record: bool,
    enum_sql_types: &[(schema_parsing::SqlTypeInfo, schema_parsing::EnumInfos)],
    db: &DB,
    table_name: &str,
) -> QueryResult<()>
where
    DB: Backend,
    for<'a> EnumType<'a>: QueryFragment<DB>,
{
    ColumnTypeName::new(ty, column_name, for_record, enum_sql_types, table_name)
        .to_sql(query_builder, db)
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
