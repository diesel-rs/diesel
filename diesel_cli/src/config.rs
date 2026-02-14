use super::find_project_root;
use crate::infer_schema_internals::TableName;
use crate::print_schema::{self, ColumnSorting, DocConfig, PrintSchemaArgs};
use clap::ArgMatches;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_regex::Serde as RegexWrapper;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::{env, fmt, fs, iter};

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub print_schema: RootPrintSchema,
    #[serde(default)]
    pub migrations_directory: Option<MigrationsDirectory>,
}

fn get_values_with_indices<'a, T: Clone + Send + Sync + 'static>(
    matches: &'a ArgMatches,
    id: &str,
) -> Result<Option<BTreeMap<usize, &'a T>>, crate::errors::Error> {
    match matches.indices_of(id) {
        Some(indices) => match matches.try_get_many::<T>(id) {
            Ok(Some(values)) => Ok(Some(indices.zip(values).collect())),
            Ok(None) => {
                unreachable!("`ids` only reports what is present")
            }
            Err(e) => Err(e.into()),
        },
        None => Ok(None),
    }
}

impl Config {
    pub fn file_path(config_file: Option<PathBuf>) -> PathBuf {
        config_file
            .or_else(|| env::var_os("DIESEL_CONFIG_FILE").map(PathBuf::from))
            .unwrap_or_else(|| find_project_root().unwrap_or_default().join("diesel.toml"))
    }

    pub fn read(config_file: Option<std::path::PathBuf>) -> Result<Self, crate::errors::Error> {
        let path = Self::file_path(config_file);

        if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
            let mut result = toml::from_str::<Self>(&content)?;
            result.set_relative_path_base(
                path.parent()
                    .expect("This is not executed in the file-system root, right?"),
            );
            Ok(result)
        } else {
            Ok(Self::default())
        }
    }

    fn set_relative_path_base(&mut self, base: &Path) {
        self.print_schema.set_relative_path_base(base);
        if let Some(ref mut migration) = self.migrations_directory {
            migration.set_relative_path_base(base);
        }
    }

    pub fn set_filter(
        mut self,
        matches: &ArgMatches,
        table_names: Vec<String>,
        only_tables: Vec<bool>,
        except_tables: Vec<bool>,
    ) -> Result<Self, crate::errors::Error> {
        if self.print_schema.has_multiple_schema {
            let selected_schema_keys =
                get_values_with_indices::<String>(matches, "SCHEMA_KEY")?.unwrap_or_default();
            let table_names_with_indices =
                get_values_with_indices::<String>(matches, "TABLE_NAME")?;
            let only_tables_with_indices = get_values_with_indices::<bool>(matches, "ONLY_TABLES")?;
            let except_tables_with_indices =
                get_values_with_indices::<bool>(matches, "EXCEPT_TABLES")?;

            for (key, boundary) in selected_schema_keys.values().map(|k| k.as_str()).zip(
                selected_schema_keys
                    .keys()
                    .copied()
                    .map(Bound::Included)
                    .zip(
                        selected_schema_keys
                            .keys()
                            .copied()
                            .skip(1)
                            .map(Bound::Excluded)
                            .chain(iter::once(Bound::Unbounded)),
                    ),
            ) {
                let print_schema = self
                    .print_schema
                    .all_configs
                    .get_mut(key)
                    .ok_or_else(|| crate::errors::Error::NoSchemaKeyFound(key.to_owned()))?;
                if let Some(table_names_with_indices) = &table_names_with_indices {
                    let table_names = table_names_with_indices
                        .range(boundary)
                        .map(|(_, v)| v.as_str())
                        .map(|table_name_regex| regex::Regex::new(table_name_regex).map(Into::into))
                        .collect::<Result<Vec<Regex>, _>>()?;
                    if table_names.is_empty() {
                        continue;
                    }
                    if except_tables_with_indices
                        .as_ref()
                        .and_then(|except_tables_with_indices| {
                            except_tables_with_indices
                                .range(boundary)
                                .nth(0)
                                .map(|v| **v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.filter = Filtering::ExceptTables(table_names);
                    } else if only_tables_with_indices
                        .as_ref()
                        .and_then(|only_tables_with_indices| {
                            only_tables_with_indices
                                .range(boundary)
                                .nth(0)
                                .map(|v| **v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.filter = Filtering::OnlyTables(table_names);
                    }
                }
            }
        } else {
            self.print_schema
                .all_configs
                .entry("default".to_string())
                .or_default()
                .set_filter(table_names, only_tables, except_tables)?;
        }
        Ok(self)
    }

    pub fn update_config(
        mut self,
        matches: &ArgMatches,
        args: PrintSchemaArgs,
    ) -> Result<Self, crate::errors::Error> {
        if self.print_schema.has_multiple_schema {
            if let Some(selected_schema_keys) =
                get_values_with_indices::<String>(matches, "SCHEMA_KEY")?
            {
                let schema_with_indices = get_values_with_indices::<String>(matches, "SCHEMA")?;
                let with_docs_with_indices = get_values_with_indices::<bool>(matches, "WITH_DOCS")?;
                let with_docs_config_with_indices =
                    get_values_with_indices::<String>(matches, "WITH_DOCS_CONFIG")?;
                let allow_tables_to_appear_in_same_query_config_with_indices =
                    get_values_with_indices::<String>(
                        matches,
                        "ALLOW_TABLES_TO_APPEAR_IN_SAME_QUERY_CONFIG",
                    )?;
                let patch_file_with_indices =
                    get_values_with_indices::<PathBuf>(matches, "PATCH_FILE")?;
                let column_sorting_with_indices =
                    get_values_with_indices::<String>(matches, "COLUMN_SORTING")?;
                let import_types_with_indices =
                    get_values_with_indices::<String>(matches, "IMPORT_TYPES")?;
                let custom_type_derives_with_indices =
                    get_values_with_indices::<String>(matches, "CUSTOM_TYPE_DERIVES")?;
                let sqlite_integer_primary_key_is_bigint_with_indices =
                    get_values_with_indices::<bool>(
                        matches,
                        "SQLITE_INTEGER_PRIMARY_KEY_IS_BIGINT",
                    )?;
                let except_custom_type_definitions_with_indices =
                    get_values_with_indices::<String>(matches, "EXCEPT_CUSTOM_TYPE_DEFINITIONS")?;

                for (key, boundary) in selected_schema_keys.values().map(|k| k.as_str()).zip(
                    selected_schema_keys
                        .keys()
                        .copied()
                        .map(Bound::Included)
                        .zip(
                            selected_schema_keys
                                .keys()
                                .copied()
                                .skip(1)
                                .map(Bound::Excluded)
                                .chain(iter::once(Bound::Unbounded)),
                        ),
                ) {
                    let print_schema =
                        self.print_schema.all_configs.get_mut(key).ok_or_else(|| {
                            crate::errors::Error::NoSchemaKeyFound(key.to_owned())
                        })?;
                    if let Some(schema) = schema_with_indices
                        .as_ref()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.as_str()))
                    {
                        print_schema.schema = Some(schema.to_owned());
                    }
                    if with_docs_with_indices
                        .as_ref()
                        .and_then(|with_docs_with_indices| {
                            with_docs_with_indices.range(boundary).nth(0).map(|v| **v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.with_docs =
                            DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment;
                    }
                    // todo: that's not correct yet
                    print_schema.include_views |= args.include_views;
                    print_schema.experimental_infer_nullable_for_views |=
                        args.experimental_infer_nullable_for_views;

                    if let Some(doc_config) = with_docs_config_with_indices
                        .as_ref()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.as_str()))
                    {
                        print_schema.with_docs = doc_config.parse().map_err(|_| {
                            crate::errors::Error::UnsupportedFeature(format!(
                                "Invalid documentation config mode: {doc_config}"
                            ))
                        })?;
                    }

                    if let Some(allow_tables_to_appear_in_same_query_config) =
                        allow_tables_to_appear_in_same_query_config_with_indices
                            .as_ref()
                            .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.as_str()))
                    {
                        print_schema.allow_tables_to_appear_in_same_query_config =
                            allow_tables_to_appear_in_same_query_config
                                .parse()
                                .map_err(|_| {
                                    crate::errors::Error::UnsupportedFeature(format!(
                                        "Invalid `allow_tables_to_appear_in_same_query!` config \
                                        mode: {allow_tables_to_appear_in_same_query_config}"
                                    ))
                                })?;
                    }

                    if let Some(sorting) = column_sorting_with_indices
                        .as_ref()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.as_str()))
                    {
                        match sorting {
                            "ordinal_position" => {
                                print_schema.column_sorting = ColumnSorting::OrdinalPosition
                            }
                            "name" => print_schema.column_sorting = ColumnSorting::Name,
                            _ => {
                                return Err(crate::errors::Error::UnsupportedFeature(format!(
                                    "Invalid column sorting mode: {sorting}"
                                )));
                            }
                        }
                    }

                    if let Some(patch_file) = patch_file_with_indices
                        .as_ref()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.as_path()))
                    {
                        print_schema.patch_file = Some(patch_file.to_owned());
                    }

                    let import_types = import_types_with_indices
                        .as_ref()
                        .map(|v| v.range(boundary).map(|v| v.1.as_str().to_owned()).collect())
                        .unwrap_or(vec![]);
                    if !import_types.is_empty() {
                        print_schema.import_types = Some(import_types);
                    }

                    if args.no_generate_missing_sql_type_definitions {
                        print_schema.generate_missing_sql_type_definitions = Some(false)
                    }

                    if let Some(excepts) = &except_custom_type_definitions_with_indices {
                        let rules = excepts
                            .range(boundary)
                            .map(|(_, v)| v.as_str())
                            .map(|rx| regex::Regex::new(rx).map(Into::into))
                            .collect::<Result<Vec<Regex>, _>>()?;

                        if !rules.is_empty() {
                            print_schema.except_custom_type_definitions = rules;
                        }
                    }

                    let custom_type_derives = custom_type_derives_with_indices
                        .as_ref()
                        .map(|v| v.range(boundary).map(|v| v.1.as_str().to_owned()).collect())
                        .unwrap_or(vec![]);
                    if !custom_type_derives.is_empty() {
                        print_schema.custom_type_derives = Some(custom_type_derives);
                    }
                    if let Some(sqlite_integer_primary_key_is_bigint) =
                        sqlite_integer_primary_key_is_bigint_with_indices
                            .as_ref()
                            .and_then(|with_docs_with_indices| {
                                with_docs_with_indices.range(boundary).nth(0).map(|v| **v.1)
                            })
                    {
                        print_schema.sqlite_integer_primary_key_is_bigint =
                            Some(sqlite_integer_primary_key_is_bigint);
                    }
                }
            }
        } else {
            let config = match self.print_schema.all_configs.entry("default".to_string()) {
                Entry::Vacant(entry) => entry.insert(PrintSchema::default()),
                Entry::Occupied(entry) => entry.into_mut(),
            };
            if let Some(schema_name) = args.schema.first() {
                config.schema = Some(schema_name.to_owned())
            }
            config.include_views |= args.include_views;
            config.experimental_infer_nullable_for_views |=
                args.experimental_infer_nullable_for_views;
            if args.with_docs.last().cloned().unwrap_or(false) {
                config.with_docs = DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment;
            }

            if let Some(docs_config) = args.with_docs_config.first() {
                config.with_docs = docs_config.to_owned();
            }

            if let Some(allow_tables_to_appear_in_same_query_config) =
                args.allow_tables_to_appear_in_same_query_config.first()
            {
                config.allow_tables_to_appear_in_same_query_config =
                    allow_tables_to_appear_in_same_query_config.to_owned();
            }

            if let Some(sorting) = args.column_sorting.first() {
                config.column_sorting = sorting.to_owned();
            }

            if let Some(path) = args.patch_file.first() {
                config.patch_file = Some(path.to_owned());
            }

            if !args.import_types.is_empty() {
                config.import_types = Some(args.import_types);
            }

            if !args.except_custom_type_definitions.is_empty() {
                config.except_custom_type_definitions = args
                    .except_custom_type_definitions
                    .into_iter()
                    .map(|x| regex::Regex::new(&x).map(Into::into))
                    .collect::<Result<Vec<Regex>, _>>()?;
            }

            if args.no_generate_missing_sql_type_definitions {
                config.generate_missing_sql_type_definitions = Some(false);
            }

            if !args.custom_type_derives.is_empty() {
                config.custom_type_derives = Some(args.custom_type_derives);
            }

            if !args.pg_domains_as_custom_types.is_empty() {
                config.pg_domains_as_custom_types = args
                    .pg_domains_as_custom_types
                    .into_iter()
                    .map(|x| regex::Regex::new(&x).map(Into::into))
                    .collect::<Result<Vec<Regex>, _>>()?;
            }

            if let Some(&last_val) = args.sqlite_integer_primary_key_is_bigint.last()
                && last_val
            {
                config.sqlite_integer_primary_key_is_bigint = Some(true);
            }
        }
        Ok(self)
    }
}

#[derive(Default, Clone, Debug)]
pub struct RootPrintSchema {
    has_multiple_schema: bool,
    pub all_configs: BTreeMap<String, PrintSchema>,
}

impl<'de> Deserialize<'de> for RootPrintSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            #[serde(flatten)]
            default_config: PrintSchema,
            #[serde(flatten)]
            other_configs: BTreeMap<String, PrintSchema>,
        }
        let Inner {
            other_configs,
            default_config,
        } = Inner::deserialize(deserializer)?;
        if other_configs.is_empty() {
            Ok(RootPrintSchema {
                has_multiple_schema: false,
                all_configs: BTreeMap::from([("default".into(), default_config)]),
            })
        } else {
            let mut other_configs = other_configs;
            other_configs
                .entry("default".to_string())
                .or_insert(default_config);
            Ok(RootPrintSchema {
                all_configs: other_configs,
                has_multiple_schema: true,
            })
        }
    }
}

impl RootPrintSchema {
    fn set_relative_path_base(&mut self, base: &Path) {
        for config in self.all_configs.values_mut() {
            config.set_relative_path_base(base);
        }
    }
}

#[derive(Default, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PrintSchema {
    #[serde(default)]
    pub file: Option<PathBuf>,
    #[serde(default)]
    pub with_docs: print_schema::DocConfig,
    #[serde(default)]
    pub allow_tables_to_appear_in_same_query_config:
        print_schema::AllowTablesToAppearInSameQueryConfig,
    #[serde(default)]
    pub filter: Filtering,
    #[serde(default)]
    pub column_sorting: ColumnSorting,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub patch_file: Option<PathBuf>,
    #[serde(default)]
    pub import_types: Option<Vec<String>>,
    #[serde(default)]
    pub generate_missing_sql_type_definitions: Option<bool>,
    #[serde(default)]
    pub except_custom_type_definitions: Vec<Regex>,
    #[serde(default)]
    pub custom_type_derives: Option<Vec<String>>,
    #[serde(default)]
    pub sqlite_integer_primary_key_is_bigint: Option<bool>,
    #[serde(default)]
    pub pg_domains_as_custom_types: Vec<Regex>,
    #[serde(default)]
    pub include_views: bool,
    #[serde(default)]
    pub experimental_infer_nullable_for_views: bool,
}

impl PrintSchema {
    pub fn generate_missing_sql_type_definitions(&self) -> bool {
        self.generate_missing_sql_type_definitions.unwrap_or(true)
    }

    pub fn schema_name(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    pub fn import_types(&self) -> Option<&[String]> {
        self.import_types.as_deref()
    }

    // it's a false positive
    // https://github.com/rust-lang/rust-clippy/issues/12856
    #[allow(clippy::needless_borrows_for_generic_args)]
    fn set_relative_path_base(&mut self, base: &Path) {
        if let Some(ref mut file) = self.file
            && file.is_relative()
        {
            *file = base.join(&file);
        }

        if let Some(ref mut patch_file) = self.patch_file
            && patch_file.is_relative()
        {
            *patch_file = base.join(&patch_file);
        }
    }

    #[cfg(any(feature = "postgres", feature = "mysql"))]
    pub fn custom_type_derives(&self) -> Vec<String> {
        let mut derives = self
            .custom_type_derives
            .as_ref()
            .map_or(Vec::new(), |derives| derives.to_vec());
        if derives
            .iter()
            .any(|item| item == "diesel::sql_types::SqlType")
        {
            derives
        } else {
            derives.push("diesel::sql_types::SqlType".to_owned());
            derives
        }
    }

    pub fn set_filter(
        &mut self,
        table_names: Vec<String>,
        only_tables: Vec<bool>,
        except_tables: Vec<bool>,
    ) -> Result<(), crate::errors::Error> {
        let table_names = table_names
            .iter()
            .map(|table_name_regex| regex::Regex::new(table_name_regex).map(Into::into))
            .collect::<Result<Vec<Regex>, _>>()?;

        let only_tables = only_tables.last().cloned().unwrap_or(false);
        let except_tables = except_tables.last().cloned().unwrap_or(false);

        if only_tables {
            self.filter = Filtering::OnlyTables(table_names)
        } else if except_tables {
            self.filter = Filtering::ExceptTables(table_names)
        }
        Ok(())
    }
}

#[derive(Default, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct MigrationsDirectory {
    pub dir: PathBuf,
}

impl MigrationsDirectory {
    fn set_relative_path_base(&mut self, base: &Path) {
        if self.dir.is_relative() {
            self.dir = base.join(&self.dir);
        }
    }
}

type Regex = RegexWrapper<::regex::Regex>;

#[derive(Clone, Debug, Default)]
pub enum Filtering {
    OnlyTables(Vec<Regex>),
    ExceptTables(Vec<Regex>),
    #[default]
    None,
}

impl Filtering {
    pub fn should_ignore_table(&self, name: &TableName) -> bool {
        use self::Filtering::*;

        match *self {
            OnlyTables(ref regexes) => !regexes.iter().any(|regex| regex.is_match(&name.sql_name)),
            ExceptTables(ref regexes) => regexes.iter().any(|regex| regex.is_match(&name.sql_name)),
            None => false,
        }
    }
}

impl<'de> Deserialize<'de> for Filtering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FilteringVisitor;

        impl<'de> Visitor<'de> for FilteringVisitor {
            type Value = Filtering;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("either only_tables or except_tables")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut only_tables = None::<Vec<Regex>>;
                let mut except_tables = None::<Vec<Regex>>;
                while let Some(key) = map.next_key::<String>()? {
                    match &key as &str {
                        "only_tables" => {
                            if only_tables.is_some() {
                                return Err(de::Error::duplicate_field("only_tables"));
                            }
                            only_tables = Some(map.next_value()?);
                        }
                        "except_tables" => {
                            if except_tables.is_some() {
                                return Err(de::Error::duplicate_field("except_tables"));
                            }
                            except_tables = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["only_tables", "except_tables"],
                            ));
                        }
                    }
                }
                match (only_tables, except_tables) {
                    (Some(t), None) => Ok(Filtering::OnlyTables(t)),
                    (None, Some(t)) => Ok(Filtering::ExceptTables(t)),
                    (None, None) => Ok(Filtering::None),
                    _ => Err(de::Error::duplicate_field("only_tables except_tables")),
                }
            }
        }

        deserializer.deserialize_map(FilteringVisitor)
    }
}
