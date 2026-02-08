use crate::config::{self, Config};
use crate::database::{Backend, InferConnection};
use crate::infer_schema_internals::*;

use clap::{ArgAction, ArgMatches, Args};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::fmt::{self, Display, Formatter, Write};
use std::io::{Write as IoWrite, stdout};
use std::{process, str};

const SCHEMA_HEADER: &str = "// @generated automatically by Diesel CLI.\n";

/// Print table definitions for database schema.
#[derive(Debug, Args)]
pub struct PrintSchemaArgs {
    /// The name of the schema.
    #[arg(id = "SCHEMA", long = "schema", short = 's', num_args = 1, action = ArgAction::Append)]
    pub schema: Vec<String>,

    /// Table names to filter.
    #[arg(id = "TABLE_NAME", num_args = 1.., action = ArgAction::Append, index = 1)]
    pub table_name: Vec<String>,

    /// Include views in the generated schema
    #[arg(id = "INCLUDE_VIEWS", long = "include-views", action = ArgAction::SetTrue)]
    pub include_views: bool,

    /// UNSTABLE: Infer nullability for view fields
    #[arg(id = "EXPERIMENTAL_INFER_NULLABLE_FOR_VIEWS", long = "experimental-infer-nullable-for-views", action = ArgAction::SetFalse)]
    pub experimental_infer_nullable_for_views: bool,

    /// Only include tables from table-name that matches regexp.
    #[arg(id = "ONLY_TABLES", long = "only-tables", short = '0', action = ArgAction::SetFalse,  value_parser = clap::value_parser!(bool))]
    pub only_tables: bool,

    /// Exclude tables from table-name that matches regex.
    #[arg(id = "EXCEPT_TABLES", long = "except-tables", short = 'e', action = ArgAction::SetFalse,  value_parser = clap::value_parser!(bool))]
    pub except_tables: bool,

    /// Render documentation comments for tables and columns.
    #[arg(id = "WITH_DOCS", long = "with-docs", action = ArgAction::SetFalse, value_parser = clap::value_parser!(bool))]
    pub with_docs: bool,

    /// Render documentation comments for tables and columns.
    #[arg(id = "WITH_DOCS_CONFIG", long = "with-docs-config", action = ArgAction::Append, value_enum, num_args = 1)]
    pub with_docs_config: Vec<DocConfig>,

    /// Group tables in allow_tables_to_appear_in_same_query!().
    #[arg(id = "ALLOW_TABLES_TO_APPEAR_IN_SAME_QUERY_CONFIG", long = "allow-tables-to-appear-in-same-query-config", action = ArgAction::Append, value_enum, num_args = 1)]
    pub allow_tables_to_appear_in_same_query_config: Vec<AllowTablesToAppearInSameQueryConfig>,

    /// Sort order for table columns.
    #[arg(
        id = "COLUMN_SORTING",
        long = "column-sorting",
        action = ArgAction::Append,
        value_enum,
        num_args = 1,
    )]
    pub column_sorting: Vec<ColumnSorting>,

    /// A unified diff file to be applied to the final schema.
    #[arg(id = "PATCH_FILE", long = "patch-file", action = ArgAction::Append, value_parser = clap::value_parser!(std::path::PathBuf), num_args = 1)]
    pub patch_file: Vec<std::path::PathBuf>,

    /// A list of types to import for every table, separated by commas.
    #[arg(id = "IMPORT_TYPES", long = "import-types", action = ArgAction::Append, value_parser , num_args = 1, number_of_values = 1)]
    pub import_types: Vec<String>,

    /// Generate SQL type definitions for types not provided by diesel
    #[arg(id = "NO_GENERATE_CUSTOM_TYPE_DEFINITIONS", long = "no-generate-missing-sql-type-definitions", action = ArgAction::SetFalse, value_parser = clap::value_parser!(bool))]
    pub no_generate_missing_sql_type_definitions: bool,

    /// A list of regexes to filter the custom types definitions generated
    #[arg(
        id = "EXCEPT_CUSTOM_TYPE_DEFINITIONS",
        long = "except-custom-type-definitions",
        num_args = 1..,
        action = clap::ArgAction::Append
    )]
    pub except_custom_type_definitions: Vec<String>,

    /// A list of derives to implement for every automatically generated SqlType in the schema, separated by commas.
    #[arg(
        id = "CUSTOM_TYPE_DERIVES",
        long = "custom-type-derives",
        num_args = 1..,
        action = clap::ArgAction::Append,
    )]
    pub custom_type_derives: Vec<String>,

    /// A regex to distinguish domain names to generate custom types for instead of relying on underlying type.
    #[arg(
            id = "PG_DOMAINS_AS_CUSTOM_TYPES",
            long = "pg-domains-as-custom-types",
            num_args = 1..,
            action = clap::ArgAction::Append
        )]
    pub pg_domains_as_custom_types: Vec<String>,

    /// Select schema key from diesel.toml, use 'default' for print_schema without key.
    #[arg(
            id = "SCHEMA_KEY",
            long = "schema-key",
            action = clap::ArgAction::Append,
            default_values_t = vec!["default".to_string()]
        )]
    pub schema_key: Vec<String>,

    /// For SQLite 3.37 and above, detect `INTEGER PRIMARY KEY` columns as `BigInt`,
    /// when the table isn't declared with `WITHOUT ROWID`.
    /// See https://www.sqlite.org/lang_createtable.html#rowid for more information.
    #[arg(
        id = "SQLITE_INTEGER_PRIMARY_KEY_IS_BIGINT",
        long = "sqlite-integer-primary-key-is-bigint",
        action = ArgAction::Append,
        num_args = 0,
        default_value = "false",
        default_missing_value = "true",
        value_parser = clap::value_parser!(bool),
    )]
    pub sqlite_integer_primary_key_is_bigint: Vec<bool>,
}

#[tracing::instrument]
pub fn run_infer_schema(
    matches: &ArgMatches,
    args: PrintSchemaArgs,
    config_file: Option<std::path::PathBuf>,
    database_url: Option<String>,
) -> Result<(), crate::errors::Error> {
    use crate::print_schema::*;

    let mut conn = InferConnection::from_maybe_url(database_url)?;
    let root_config = Config::read(config_file)?
        .set_filter(
            matches,
            args.table_name.clone(),
            args.only_tables,
            args.except_tables,
        )?
        .update_config(matches, args)?
        .print_schema;
    for config in root_config.all_configs.values() {
        run_print_schema(&mut conn, config, &mut stdout())?;
    }

    Ok(())
}

/// How to sort columns when querying the table schema.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, clap::ValueEnum)]
pub enum ColumnSorting {
    /// Order by ordinal position
    #[serde(rename = "ordinal_position")]
    #[default]
    OrdinalPosition,
    /// Order by column name
    #[serde(rename = "name")]
    Name,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum DocConfig {
    DatabaseCommentsFallbackToAutoGeneratedDocComment,
    OnlyDatabaseComments,
    #[default]
    NoDocComments,
}

/// How to group tables in `allow_tables_to_appear_in_same_query!()`.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, clap::ValueEnum)]
pub enum AllowTablesToAppearInSameQueryConfig {
    /// Group by foreign key relations
    #[serde(rename = "fk_related_tables")]
    FkRelatedTables,
    /// List all tables in invocation
    #[serde(rename = "all_tables")]
    #[default]
    AllTables,
    /// Don't generate any invocation
    #[serde(rename = "none")]
    None,
}

pub fn run_print_schema<W: IoWrite>(
    connection: &mut InferConnection,
    config: &config::PrintSchema,
    output: &mut W,
) -> Result<(), crate::errors::Error> {
    let schema = output_schema(connection, config)?;

    output
        .write_all(schema.as_bytes())
        .map_err(|e| crate::errors::Error::IoError(e, None))?;
    Ok(())
}

fn common_diesel_types(types: &mut HashSet<&str>) {
    types.insert("Bool");
    types.insert("Integer");
    types.insert("SmallInt");
    types.insert("BigInt");
    types.insert("Binary");
    types.insert("Text");
    types.insert("Double");
    types.insert("Float");
    types.insert("Numeric");
    types.insert("Timestamp");
    types.insert("Date");
    types.insert("Time");

    // hidden type defs
    types.insert("Float4");
    types.insert("Smallint");
    types.insert("Int2");
    types.insert("Int4");
    types.insert("Int8");
    types.insert("Bigint");
    types.insert("Float8");
    types.insert("Decimal");
    types.insert("VarChar");
    types.insert("Varchar");
    types.insert("Char");
    types.insert("Tinytext");
    types.insert("Mediumtext");
    types.insert("Longtext");
    types.insert("Tinyblob");
    types.insert("Blob");
    types.insert("Mediumblob");
    types.insert("Longblob");
    types.insert("Varbinary");
    types.insert("Bit");
}

#[cfg(feature = "postgres")]
fn pg_diesel_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    types.insert("Cidr");
    types.insert("Citext");
    types.insert("Inet");
    types.insert("Jsonb");
    types.insert("MacAddr");
    types.insert("MacAddr8");
    types.insert("Money");
    types.insert("Oid");
    types.insert("Range");
    types.insert("Timestamptz");
    types.insert("Uuid");
    types.insert("Json");
    types.insert("PgLsn");
    types.insert("Record");
    types.insert("Interval");

    // hidden type defs
    types.insert("Int4range");
    types.insert("Int8range");
    types.insert("Daterange");
    types.insert("Numrange");
    types.insert("Tsrange");
    types.insert("Tstzrange");
    types.insert("Int4multirange");
    types.insert("Int8multirange");
    types.insert("Datemultirange");
    types.insert("Nummultirange");
    types.insert("Tsmultirange");
    types.insert("Tstzmultirange");
    types.insert("SmallSerial");
    types.insert("BigSerial");
    types.insert("Serial");
    types.insert("Bytea");
    types.insert("Bpchar");
    types.insert("Macaddr");
    types.insert("Macaddr8");

    common_diesel_types(&mut types);
    types
}

#[cfg(feature = "mysql")]
fn mysql_diesel_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    common_diesel_types(&mut types);

    types.insert("TinyInt");
    types.insert("Tinyint");
    types.insert("Datetime");
    types.insert("Json");
    types
}

#[cfg(feature = "sqlite")]
fn sqlite_diesel_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    common_diesel_types(&mut types);
    types
}

#[tracing::instrument(skip(connection))]
pub fn output_schema(
    connection: &mut InferConnection,
    config: &config::PrintSchema,
) -> Result<String, crate::errors::Error> {
    let backend = Backend::for_connection(connection);
    let unfiltered_table_names = load_table_names(connection, config.schema_name())?;
    let table_names = filter_table_names(
        &unfiltered_table_names,
        &config.filter,
        config.include_views,
    );

    let foreign_keys = load_foreign_key_constraints(connection, config.schema_name())?;
    let safe_tables =
        &filter_column_structure(&table_names, SupportedQueryRelationStructures::Table);
    let foreign_keys_for_allow_tables =
        filter_foreign_keys_for_grouping(&foreign_keys, safe_tables);
    let duplicate_foreign_keys = duplicated_foreign_keys(&foreign_keys);
    let foreign_keys_for_joinable =
        remove_unsafe_foreign_keys_for_codegen(connection, &foreign_keys, safe_tables);
    let foreign_keys_for_joinable =
        remove_duplicated_foreign_keys(&foreign_keys_for_joinable, &duplicate_foreign_keys);

    let resolver = SchemaResolverImpl::new(connection, table_names, config, unfiltered_table_names);
    let data = resolver.resolve_query_relations()?;

    let columns_custom_types = if config.generate_missing_sql_type_definitions() {
        let diesel_provided_types = match backend {
            #[cfg(feature = "postgres")]
            Backend::Pg => pg_diesel_types(),
            #[cfg(feature = "sqlite")]
            Backend::Sqlite => sqlite_diesel_types(),
            #[cfg(feature = "mysql")]
            Backend::Mysql => mysql_diesel_types(),
        };

        Some(
            data.iter()
                .map(|cd| {
                    cd.columns()
                        .iter()
                        .map(|c| {
                            Some(&c.ty)
                                .filter(|ty| !diesel_provided_types.contains(ty.rust_name.as_str()))
                                // Skip types that are that match the regexes in the configuration
                                .filter(|ty| {
                                    !config
                                        .except_custom_type_definitions
                                        .iter()
                                        .any(|rx| rx.is_match(ty.rust_name.as_str()))
                                })
                                .map(|ty| match backend {
                                    #[cfg(feature = "postgres")]
                                    Backend::Pg => ty.clone(),
                                    #[cfg(feature = "sqlite")]
                                    Backend::Sqlite => ty.clone(),
                                    #[cfg(feature = "mysql")]
                                    Backend::Mysql => {
                                        // For MySQL we generate custom types for unknown types that
                                        // are dedicated to the column
                                        use heck::ToUpperCamelCase;

                                        ColumnType {
                                            rust_name: format!(
                                                "{} {} {}",
                                                &cd.table_name().rust_name,
                                                &c.rust_name,
                                                &ty.rust_name
                                            )
                                            .to_upper_camel_case(),
                                            ..ty.clone()
                                        }
                                    }
                                })
                        })
                        .collect::<Vec<Option<ColumnType>>>()
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    let definitions = QueryRelationDefinitions {
        data,
        fk_constraints_for_joinable: foreign_keys_for_joinable,
        fk_constraints_for_allow_tables: foreign_keys_for_allow_tables,
        with_docs: config.with_docs,
        allow_tables_to_appear_in_same_query_config: config
            .allow_tables_to_appear_in_same_query_config,
        custom_types_for_tables: columns_custom_types.map(|custom_types_sorted| {
            CustomTypesForTables {
                backend,
                types_overrides_sorted: custom_types_sorted,
                with_docs: match config.with_docs {
                    DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment => true,
                    DocConfig::OnlyDatabaseComments | DocConfig::NoDocComments => false,
                },
                #[cfg(any(feature = "postgres", feature = "mysql"))]
                derives: config.custom_type_derives(),
            }
        }),
        import_types: config.import_types(),
    };

    let mut out = String::new();
    writeln!(out, "{SCHEMA_HEADER}")?;
    if let Some(schema_name) = config.schema_name() {
        write!(out, "{}", ModuleDefinition(schema_name, definitions))?;
    } else {
        if let Some(ref custom_types_for_tables) = definitions.custom_types_for_tables {
            write!(
                out,
                "{}",
                CustomTypesForTablesForDisplay {
                    custom_types: custom_types_for_tables,
                    tables: &definitions.data
                }
            )?;
        }

        write!(out, "{definitions}")?;
    }

    out = match format_schema(&out) {
        Ok(schema) => schema,
        Err(err) => {
            tracing::warn!(
                "Couldn't format schema. Exporting unformatted schema ({:?})",
                err
            );
            out
        }
    };

    if let Some(ref patch_file) = config.patch_file {
        tracing::info!(
            ?patch_file,
            "Found patch file to apply to the generated schema"
        );
        tracing::trace!(?out, "Schema before applying patch file");
        let patch = match std::fs::read_to_string(patch_file) {
            Ok(patch) => patch,
            Err(e) => {
                eprintln!(
                    "Failed to read patch file at {}: {}",
                    patch_file.display(),
                    e
                );
                return Err(crate::errors::Error::IoError(e, Some(patch_file.clone())));
            }
        };
        let patch = diffy::Patch::from_str(&patch)?;

        out = diffy::apply(&out, &patch)?;
    }

    Ok(out)
}

pub fn format_schema(schema: &str) -> Result<String, crate::errors::Error> {
    use crate::errors::Error;
    // Inject schema through rustfmt stdin and get the formatted output
    let mut child = process::Command::new("rustfmt")
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
        .map_err(|err| Error::RustFmtFail(format!("Failed to launch child process ({err})")))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .expect("we can always get the stdin from the child process");

        stdin.write_all(schema.as_bytes()).map_err(|err| {
            Error::RustFmtFail(format!("Failed to send schema to rustfmt ({err})"))
        })?;
        // the inner scope makes it so stdin gets dropped here
    }

    let output = child
        .wait_with_output()
        .map_err(|err| Error::RustFmtFail(format!("Couldn't wait for child ({err})")))?;

    // in cases rustfmt isn't installed, it will fail with
    // 'error: 'rustfmt' is not installed for ...'
    // this catches that error
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).expect("rustfmt output is valid utf-8");
        return Err(Error::RustFmtFail(format!("rustfmt error ({stderr})")));
    }

    let out = String::from_utf8(output.stdout).expect("rustfmt output is valid utf-8");
    Ok(out)
}

struct CustomTypesForTables {
    backend: Backend,
    // To be zipped with tables then columns
    types_overrides_sorted: Vec<Vec<Option<ColumnType>>>,
    with_docs: bool,
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    derives: Vec<String>,
}

pub struct CustomTypesForTablesForDisplay<'a> {
    custom_types: &'a CustomTypesForTables,
    tables: &'a [QueryRelationData],
}

#[allow(clippy::print_in_format_impl)]
impl Display for CustomTypesForTablesForDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.custom_types.backend {
            #[cfg(feature = "postgres")]
            Backend::Pg => {
                let _ = &self.tables;
                let mut types_to_generate: Vec<&ColumnType> = self
                    .custom_types
                    .types_overrides_sorted
                    .iter()
                    .flatten()
                    .flatten()
                    .collect();
                if types_to_generate.is_empty() {
                    return Ok(());
                }
                types_to_generate
                    .sort_unstable_by_key(|column_type| column_type.rust_name.as_str());
                // On PG we expect that there may be duplicates because types names are not made
                // specific to the column, unlike MySQL
                types_to_generate.dedup_by_key(|column_type| column_type.rust_name.as_str());

                if self.custom_types.with_docs {
                    writeln!(f, "/// A module containing custom SQL type definitions")?;
                    writeln!(f, "///")?;
                    writeln!(f, "/// (Automatically generated by Diesel.)")?;
                }
                let mut out = PadAdapter::new(f);
                writeln!(out, "pub mod sql_types {{")?;

                for (idx, &ct) in types_to_generate.iter().enumerate() {
                    if idx != 0 {
                        writeln!(out)?;
                    }
                    if self.custom_types.with_docs {
                        if let Some(ref schema) = ct.schema {
                            writeln!(out, "/// The `{}.{}` SQL type", schema, ct.sql_name)?;
                        } else {
                            writeln!(out, "/// The `{}` SQL type", ct.sql_name)?;
                        }
                        writeln!(out, "///")?;
                        writeln!(out, "/// (Automatically generated by Diesel.)")?;
                    }
                    writeln!(out, "#[derive({})]", self.custom_types.derives.join(", "))?;
                    if let Some(ref schema) = ct.schema {
                        writeln!(
                            out,
                            "#[diesel(postgres_type(name = \"{}\", schema = \"{}\"))]",
                            ct.sql_name, schema
                        )?;
                    } else {
                        writeln!(out, "#[diesel(postgres_type(name = \"{}\"))]", ct.sql_name)?;
                    }
                    writeln!(out, "pub struct {};", ct.rust_name)?;
                }

                writeln!(f, "}}\n")?;
                Ok(())
            }
            #[cfg(feature = "sqlite")]
            Backend::Sqlite => {
                let _ = (&f, self.custom_types.with_docs, &self.tables);

                let mut types_to_generate: Vec<&ColumnType> = self
                    .custom_types
                    .types_overrides_sorted
                    .iter()
                    .flatten()
                    .flatten()
                    .collect();
                types_to_generate
                    .sort_unstable_by_key(|column_type| column_type.rust_name.as_str());

                if types_to_generate.is_empty() {
                    return Ok(());
                }
                for t in &types_to_generate {
                    eprintln!("Encountered unknown type for Sqlite: {}", t.sql_name);
                }
                unreachable!(
                    "Diesel only support a closed set of types for Sqlite. \
                     If you ever see this error message please open an \
                     issue at https://github.com/diesel-rs/diesel containing \
                     a dump of your schema definition."
                )
            }
            #[cfg(feature = "mysql")]
            Backend::Mysql => {
                let CustomTypesForTables {
                    types_overrides_sorted,
                    with_docs,
                    derives,
                    ..
                } = self.custom_types;
                let mut types_to_generate: Vec<(&ColumnType, &TableName, &ColumnDefinition)> =
                    types_overrides_sorted
                        .iter()
                        .zip(self.tables)
                        .flat_map(|(ct, t)| {
                            ct.iter().zip(t.columns()).filter_map(move |(ct, c)| {
                                ct.as_ref().map(|ct| (ct, t.table_name(), c))
                            })
                        })
                        .collect();
                if types_to_generate.is_empty() {
                    return Ok(());
                }
                types_to_generate.sort_by_key(|(column_type, _, _)| column_type.rust_name.as_str());

                if *with_docs {
                    writeln!(f, "/// A module containing custom SQL type definitions")?;
                    writeln!(f, "///")?;
                    writeln!(f, "/// (Automatically generated by Diesel.)")?;
                }

                let mut out = PadAdapter::new(f);
                writeln!(out, "pub mod sql_types {{")?;

                for (idx, &(custom_type, table, column)) in types_to_generate.iter().enumerate() {
                    if idx != 0 {
                        writeln!(out)?;
                    }

                    if self.custom_types.with_docs {
                        writeln!(
                            out,
                            "/// The `{}` SQL type for the\n\
                             /// [`{tbl}::{col}`](super::{tbl}::{col})) column",
                            custom_type.sql_name,
                            tbl = table.rust_name,
                            col = column.rust_name,
                        )?;
                        writeln!(out, "///")?;
                        writeln!(out, "/// (Automatically generated by Diesel.)")?;
                    }

                    writeln!(out, "#[derive({})]", derives.join(", "))?;

                    let mysql_name = {
                        let mut c = custom_type.sql_name.chars();

                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().chain(c).collect::<String>(),
                        }
                    };

                    writeln!(out, "#[diesel(mysql_type(name = \"{mysql_name}\"))]")?;
                    writeln!(out, "pub struct {};", custom_type.rust_name)?;
                }

                writeln!(f, "}}\n")?;
                Ok(())
            }
        }
    }
}

struct ModuleDefinition<'a>(&'a str, QueryRelationDefinitions<'a>);

impl Display for ModuleDefinition<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        {
            let mut out = PadAdapter::new(f);
            writeln!(out, "pub mod {} {{", self.0)?;
            if let Some(ref custom_types_for_tables) = self.1.custom_types_for_tables {
                write!(
                    out,
                    "{}",
                    CustomTypesForTablesForDisplay {
                        custom_types: custom_types_for_tables,
                        tables: &self.1.data
                    }
                )?;
            }
            write!(out, "{}", self.1)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

struct QueryRelationDefinitions<'a> {
    data: Vec<QueryRelationData>,
    fk_constraints_for_joinable: Vec<ForeignKeyConstraint>,
    fk_constraints_for_allow_tables: Vec<ForeignKeyConstraint>,
    with_docs: DocConfig,
    allow_tables_to_appear_in_same_query_config: AllowTablesToAppearInSameQueryConfig,
    import_types: Option<&'a [String]>,
    custom_types_for_tables: Option<CustomTypesForTables>,
}

impl<'a> Display for QueryRelationDefinitions<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut is_first = true;
        for (table_idx, table) in self.data.iter().enumerate() {
            if is_first {
                is_first = false;
            } else {
                writeln!(f)?;
            }
            writeln!(
                f,
                "{}",
                QueryRelationDefinition {
                    table,
                    with_docs: self.with_docs,
                    import_types: self.import_types,
                    custom_type_overrides: self
                        .custom_types_for_tables
                        .as_ref()
                        .map(|cts| cts.types_overrides_sorted[table_idx].as_slice())
                }
            )?;
        }

        if !self.fk_constraints_for_joinable.is_empty() {
            writeln!(f)?;
        }

        for foreign_key in &self.fk_constraints_for_joinable {
            writeln!(f, "{}", Joinable(foreign_key))?;
        }

        let table_groups = match self.allow_tables_to_appear_in_same_query_config {
            AllowTablesToAppearInSameQueryConfig::FkRelatedTables => foreign_key_table_groups(
                self.data
                    .iter()
                    .filter_map(|t| match t {
                        QueryRelationData::View(_) => None,
                        QueryRelationData::Table(table_data) => Some(table_data),
                    })
                    .collect(),
                &self.fk_constraints_for_allow_tables,
            ),
            AllowTablesToAppearInSameQueryConfig::AllTables => {
                vec![self.data.iter().map(|table| table.table_name()).collect()]
            }
            AllowTablesToAppearInSameQueryConfig::None => vec![],
        };
        for (table_group_index, table_group) in table_groups
            .into_iter()
            .filter(|table_group| table_group.len() >= 2)
            .enumerate()
        {
            if table_group_index == 0 {
                writeln!(f)?;
            }
            write!(f, "diesel::allow_tables_to_appear_in_same_query!(")?;
            {
                let mut out = PadAdapter::new(f);
                writeln!(out)?;
                for table in table_group {
                    write!(out, "{},", table.rust_name)?;
                }
            }
            writeln!(f, ");")?;
        }

        Ok(())
    }
}

/// Calculates groups of tables that are related by foreign key.
///
/// Given the graph of all tables and their foreign key relations, this returns the set of connected
/// components of that graph.
fn foreign_key_table_groups<'a>(
    tables: Vec<&'a TableData>,
    fk_constraints: &'a [ForeignKeyConstraint],
) -> Vec<Vec<&'a TableName>> {
    let mut visited = BTreeSet::new();
    let mut components = vec![];

    // Find connected components in table graph. For the intended purpose of this function, we treat
    // the foreign key relation as being symmetrical, i.e. we are operating on the undirected graph.
    //
    // The algorithm is not optimized and suffers from repeated lookups in the foreign key list, but
    // it should be sufficient for typical table counts from a few dozen up to a few hundred tables.
    for table in tables {
        let name = &table.name;
        if visited.contains(name) {
            // This table is already part of another connected component.
            continue;
        }

        visited.insert(name);
        let mut component = vec![];
        let mut pending = vec![name];

        // Start a depth-first search with the current table name, walking the foreign key relations
        // in both directions.
        while let Some(name) = pending.pop() {
            component.push(name);

            let mut visit = |related_name: &'a TableName| {
                if visited.insert(related_name) {
                    pending.push(related_name);
                }
            };

            // Visit all remaining child tables that have this table as parent.
            for foreign_key in fk_constraints.iter().filter(|fk| fk.parent_table == *name) {
                visit(&foreign_key.child_table);
            }

            // Visit all remaining parent tables that have this table as child.
            for foreign_key in fk_constraints.iter().filter(|fk| fk.child_table == *name) {
                visit(&foreign_key.parent_table);
            }
        }

        // The component contains all tables that are reachable in either direction from the current
        // table. Sort that list by table name to ensure a stable output that does not depend on the
        // algorithm's specific implementation.
        component.sort();

        components.push(component);
    }

    // Sort the list of components to ensure a stable output that does not depend on the algorithm's
    // specific implementation. This sorts the list of components by the name of the first tables in
    // each component.
    components.sort();

    components
}

struct QueryRelationDefinition<'a> {
    table: &'a QueryRelationData,
    with_docs: DocConfig,
    import_types: Option<&'a [String]>,
    custom_type_overrides: Option<&'a [Option<ColumnType>]>,
}

fn write_doc_comments(out: &mut impl fmt::Write, doc: &str) -> fmt::Result {
    for line in doc.lines() {
        let line = line.trim();
        writeln!(out, "///{}{}", if line.is_empty() { "" } else { " " }, line)?;
    }
    Ok(())
}

impl<'a> Display for QueryRelationDefinition<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.table {
            QueryRelationData::Table(_) => write!(f, "diesel::table! {{")?,
            QueryRelationData::View(_) => write!(f, "diesel::view! {{")?,
        }

        {
            let mut out = PadAdapter::new(f);
            writeln!(out)?;

            let mut has_written_import = false;
            if let Some(types) = self.import_types {
                for import in types {
                    writeln!(out, "use {import};")?;
                    has_written_import = true;
                }
            }

            #[cfg(any(feature = "mysql", feature = "postgres"))]
            {
                let mut already_imported_custom_types: HashSet<&str> = HashSet::new();
                for ct in self
                    .custom_type_overrides
                    .iter()
                    .copied()
                    .flatten()
                    .filter_map(|opt| opt.as_ref())
                {
                    if already_imported_custom_types.insert(&ct.rust_name) {
                        if !has_written_import {
                            writeln!(out, "use diesel::sql_types::*;")?;
                        }
                        writeln!(out, "use super::sql_types::{};", ct.rust_name)?;
                        has_written_import = true;
                    }
                }
            }

            #[cfg(not(any(feature = "mysql", feature = "postgres")))]
            let _ = self.custom_type_overrides;

            if has_written_import {
                writeln!(out)?;
            }

            let full_sql_name = self.table.table_name().full_sql_name();

            match self.with_docs {
                DocConfig::NoDocComments => {}
                DocConfig::OnlyDatabaseComments => {
                    if let Some(comment) = self.table.comment().as_deref() {
                        write_doc_comments(&mut out, comment)?;
                    }
                }
                DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment => {
                    if let Some(comment) = self.table.comment().as_deref() {
                        write_doc_comments(&mut out, comment)?;
                    } else {
                        write_doc_comments(
                            &mut out,
                            &format!(
                                "Representation of the `{full_sql_name}` {}.

                                (Automatically generated by Diesel.)",
                                self.table.relation_type()
                            ),
                        )?;
                    }
                }
            }

            if self.table.table_name().rust_name != self.table.table_name().sql_name {
                writeln!(out, r#"#[sql_name = "{full_sql_name}"]"#,)?;
            }

            write!(out, "{} ", self.table.table_name())?;

            if let QueryRelationData::Table(t) = self.table {
                write!(out, "(")?;
                for (i, pk) in t.primary_key.iter().enumerate() {
                    if i != 0 {
                        write!(out, ", ")?;
                    }
                    write!(out, "{pk}")?;
                }
                write!(out, ") ")?;
            }

            write!(
                out,
                "{}",
                ColumnDefinitions {
                    columns: self.table.columns(),
                    with_docs: self.with_docs,
                    table_full_sql_name: &full_sql_name,
                    custom_type_overrides: self.custom_type_overrides,
                    relation_type: self.table.relation_type(),
                }
            )?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

struct ColumnDefinitions<'a> {
    columns: &'a [ColumnDefinition],
    with_docs: DocConfig,
    table_full_sql_name: &'a str,
    custom_type_overrides: Option<&'a [Option<ColumnType>]>,
    relation_type: &'static str,
}

impl Display for ColumnDefinitions<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        {
            let mut out = PadAdapter::new(f);
            writeln!(out, "{{")?;
            for (column_idx, column) in self.columns.iter().enumerate() {
                let column_type = self
                    .custom_type_overrides
                    .and_then(|ct| ct[column_idx].as_ref())
                    .unwrap_or(&column.ty);

                match self.with_docs {
                    DocConfig::NoDocComments => {}
                    DocConfig::OnlyDatabaseComments => {
                        if let Some(comment) = column.comment.as_deref() {
                            write_doc_comments(&mut out, comment)?;
                        }
                    }
                    DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment => {
                        if let Some(comment) = column.comment.as_deref() {
                            write_doc_comments(&mut out, comment)?;
                        } else {
                            write_doc_comments(
                                &mut out,
                                &format!(
                                    "The `{}` column of the `{}` {}.

                                    Its SQL type is `{}`.

                                    (Automatically generated by Diesel.)",
                                    column.sql_name,
                                    self.table_full_sql_name,
                                    self.relation_type,
                                    column_type,
                                ),
                            )?;
                        }
                    }
                }

                // Write out attributes
                if column.rust_name != column.sql_name {
                    writeln!(out, r#"#[sql_name = "{}"]"#, column.sql_name)?;
                }
                if let Some(max_length) = column.ty.max_length {
                    writeln!(out, r#"#[max_length = {max_length}]"#)?;
                }

                writeln!(out, "{} -> {},", column.rust_name, column_type)?;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

struct Joinable<'a>(&'a ForeignKeyConstraint);

impl Display for Joinable<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let child_table_name = &self.0.child_table.rust_name;

        let parent_table_name = &self.0.parent_table.rust_name;

        write!(
            f,
            "diesel::joinable!({} -> {} ({}));",
            child_table_name, parent_table_name, self.0.foreign_key_columns_rust[0],
        )
    }
}

/// Lifted directly from libcore/fmt/builders.rs
struct PadAdapter<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    on_newline: bool,
}

impl<'a, 'b: 'a> PadAdapter<'a, 'b> {
    fn new(fmt: &'a mut Formatter<'b>) -> PadAdapter<'a, 'b> {
        PadAdapter {
            fmt,
            on_newline: false,
        }
    }
}

impl<'a, 'b: 'a> Write for PadAdapter<'a, 'b> {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        while !s.is_empty() {
            let on_newline = self.on_newline;

            let split = match s.find('\n') {
                Some(pos) => {
                    self.on_newline = true;
                    pos + 1
                }
                None => {
                    self.on_newline = false;
                    s.len()
                }
            };

            let to_write = &s[..split];
            if on_newline && to_write != "\n" {
                self.fmt.write_str("    ")?;
            }
            self.fmt.write_str(to_write)?;

            s = &s[split..];
        }

        Ok(())
    }
}

impl DocConfig {
    pub const VARIANTS_STR: &'static [&'static str] = &[
        "database-comments-fallback-to-auto-generated-doc-comment",
        "only-database-comments",
        "no-doc-comments",
    ];
}

impl<'de> Deserialize<'de> for DocConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DocConfigVisitor;
        impl serde::de::Visitor<'_> for DocConfigVisitor {
            type Value = DocConfig;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "a boolean or one of the following: {:?}",
                    DocConfig::VARIANTS_STR
                )
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match v {
                    true => DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment,
                    false => DocConfig::NoDocComments,
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match v {
                    "database-comments-fallback-to-auto-generated-doc-comment" => {
                        DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment
                    }
                    "only-database-comments" => DocConfig::OnlyDatabaseComments,
                    "no-doc-comments" => DocConfig::NoDocComments,
                    _ => {
                        return Err(serde::de::Error::unknown_variant(
                            v,
                            DocConfig::VARIANTS_STR,
                        ));
                    }
                })
            }
        }

        deserializer.deserialize_any(DocConfigVisitor)
    }
}

impl str::FromStr for DocConfig {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "database-comments-fallback-to-auto-generated-doc-comment" => {
                DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment
            }
            "only-database-comments" => DocConfig::OnlyDatabaseComments,
            "no-doc-comments" => DocConfig::NoDocComments,
            _ => {
                return Err("Unknown variant for doc config, expected one of: \
                    `database-comments-fallback-to-auto-generated-doc-comment`, \
                    `only-database-comments`, \
                    `no-doc-comments`");
            }
        })
    }
}

impl str::FromStr for AllowTablesToAppearInSameQueryConfig {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "fk_related_tables" => AllowTablesToAppearInSameQueryConfig::FkRelatedTables,
            "all_tables" => AllowTablesToAppearInSameQueryConfig::AllTables,
            "none" => AllowTablesToAppearInSameQueryConfig::None,
            _ => {
                return Err(
                    "Unknown variant for `allow_tables_to_appear_in_same_query!` config \
                    mode, expected one of: \
                    `fk_related_tables`, \
                    `all_tables`",
                );
            }
        })
    }
}
