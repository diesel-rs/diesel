use super::find_project_root;
use crate::infer_schema_internals::TableName;
use crate::print_schema::ColumnSorting;
use crate::print_schema::{self, DocConfig};
use clap::ArgMatches;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_regex::Serde as RegexWrapper;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::{env, fmt};
use std::{fs, iter};

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub print_schema: RootPrintSchema,
    #[serde(default)]
    pub migrations_directory: Option<MigrationsDirectory>,
}

fn get_values_with_indices<T: Clone + Send + Sync + 'static>(
    matches: &ArgMatches,
    id: &str,
) -> Result<Option<BTreeMap<usize, T>>, crate::errors::Error> {
    match matches.indices_of(id) {
        Some(indices) => match matches.try_get_many::<T>(id) {
            Ok(Some(values)) => Ok(Some(
                indices
                    .zip(values)
                    .map(|(index, value)| (index, value.clone()))
                    .collect(),
            )),
            Ok(None) => {
                unreachable!("`ids` only reports what is present")
            }
            Err(e) => Err(e.into()),
        },
        None => Ok(None),
    }
}

impl Config {
    pub fn file_path(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("CONFIG_FILE")
            .cloned()
            .or_else(|| env::var_os("DIESEL_CONFIG_FILE").map(PathBuf::from))
            .unwrap_or_else(|| find_project_root().unwrap_or_default().join("diesel.toml"))
    }

    pub fn read(matches: &ArgMatches) -> Result<Self, crate::errors::Error> {
        let path = Self::file_path(matches);

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

    pub fn set_filter(mut self, matches: &ArgMatches) -> Result<Self, crate::errors::Error> {
        if self.print_schema.has_multiple_schema {
            let selected_schema_keys =
                get_values_with_indices::<String>(matches, "schema-key")?.unwrap_or_default();
            let table_names_with_indices =
                get_values_with_indices::<String>(matches, "table-name")?;
            let only_tables_with_indices = get_values_with_indices::<bool>(matches, "only-tables")?;
            let except_tables_with_indices =
                get_values_with_indices::<bool>(matches, "except-tables")?;

            for (key, boundary) in selected_schema_keys.values().cloned().zip(
                selected_schema_keys
                    .keys()
                    .cloned()
                    .map(Bound::Included)
                    .zip(
                        selected_schema_keys
                            .keys()
                            .cloned()
                            .skip(1)
                            .map(Bound::Excluded)
                            .chain(iter::once(Bound::Unbounded)),
                    ),
            ) {
                let print_schema = self
                    .print_schema
                    .all_configs
                    .get_mut(&key)
                    .ok_or(crate::errors::Error::NoSchemaKeyFound(key))?;
                if let Some(table_names_with_indices) = table_names_with_indices.clone() {
                    let table_names = table_names_with_indices
                        .range(boundary)
                        .map(|(_, v)| v.clone())
                        .map(|table_name_regex| {
                            regex::Regex::new(&table_name_regex).map(Into::into)
                        })
                        .collect::<Result<Vec<Regex>, _>>()?;
                    if table_names.is_empty() {
                        continue;
                    }
                    if only_tables_with_indices
                        .clone()
                        .and_then(|only_tables_with_indices| {
                            only_tables_with_indices
                                .range(boundary)
                                .nth(0)
                                .map(|v| *v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.filter = Filtering::OnlyTables(table_names.clone());
                    }
                    if except_tables_with_indices
                        .clone()
                        .and_then(|except_tables_with_indices| {
                            except_tables_with_indices
                                .range(boundary)
                                .nth(0)
                                .map(|v| *v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.filter = Filtering::ExceptTables(table_names);
                    }
                }
            }
        } else {
            let print_schema = self
                .print_schema
                .all_configs
                .entry("default".to_string())
                .or_insert(PrintSchema::default().set_filter(matches)?);
            let print_schema = print_schema.clone().set_filter(matches)?;
            self.print_schema
                .all_configs
                .entry("default".to_string())
                .and_modify(|v| *v = print_schema);
        }
        Ok(self)
    }

    pub fn update_config(mut self, matches: &ArgMatches) -> Result<Self, crate::errors::Error> {
        if self.print_schema.has_multiple_schema {
            if let Some(selected_schema_keys) =
                get_values_with_indices::<String>(matches, "schema-key")?
            {
                let schema_with_indices = get_values_with_indices::<String>(matches, "schema")?;
                let with_docs_with_indices = get_values_with_indices::<bool>(matches, "with-docs")?;
                let with_docs_config_with_indices =
                    get_values_with_indices::<String>(matches, "with-docs-config")?;
                let patch_file_with_indices =
                    get_values_with_indices::<PathBuf>(matches, "patch-file")?;
                let column_sorting_with_indices =
                    get_values_with_indices::<String>(matches, "column-sorting")?;
                let import_types_with_indices =
                    get_values_with_indices::<String>(matches, "import-types")?;
                let generate_custom_type_definitions_with_indices =
                    get_values_with_indices::<bool>(matches, "generate-custom-type-definitions")?;
                let custom_type_derives_with_indices =
                    get_values_with_indices::<String>(matches, "custom-type-derives")?;
                let sqlite_integer_primary_key_is_bigint_with_indices =
                    get_values_with_indices::<bool>(
                        matches,
                        "sqlite-integer-primary-key-is-bigint",
                    )?;
                let except_custom_type_definitions_with_indices =
                    get_values_with_indices::<Vec<Regex>>(
                        matches,
                        "except-custom-type-definitions",
                    )?;

                for (key, boundary) in selected_schema_keys.values().cloned().zip(
                    selected_schema_keys
                        .keys()
                        .cloned()
                        .map(Bound::Included)
                        .zip(
                            selected_schema_keys
                                .keys()
                                .cloned()
                                .skip(1)
                                .map(Bound::Excluded)
                                .chain(iter::once(Bound::Unbounded)),
                        ),
                ) {
                    let print_schema = self
                        .print_schema
                        .all_configs
                        .get_mut(&key)
                        .ok_or(crate::errors::Error::NoSchemaKeyFound(key))?;
                    if let Some(schema) = schema_with_indices
                        .clone()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.clone()))
                    {
                        print_schema.schema = Some(schema)
                    }
                    if with_docs_with_indices
                        .clone()
                        .and_then(|with_docs_with_indices| {
                            with_docs_with_indices.range(boundary).nth(0).map(|v| *v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.with_docs =
                            DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment;
                    }

                    if let Some(doc_config) = with_docs_config_with_indices
                        .clone()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.clone()))
                    {
                        print_schema.with_docs = doc_config.parse().map_err(|_| {
                            crate::errors::Error::UnsupportedFeature(format!(
                                "Invalid documentation config mode: {doc_config}"
                            ))
                        })?;
                    }

                    if let Some(sorting) = column_sorting_with_indices
                        .clone()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.clone()))
                    {
                        match sorting.as_str() {
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
                        .clone()
                        .and_then(|v| v.range(boundary).nth(0).map(|v| v.1.clone()))
                    {
                        print_schema.patch_file = Some(patch_file);
                    }

                    let import_types = import_types_with_indices
                        .clone()
                        .map(|v| v.range(boundary).map(|v| v.1.clone()).collect())
                        .unwrap_or(vec![]);
                    if !import_types.is_empty() {
                        print_schema.import_types = Some(import_types);
                    }

                    if generate_custom_type_definitions_with_indices
                        .clone()
                        .and_then(|generate_custom_type_definitions_with_indices| {
                            generate_custom_type_definitions_with_indices
                                .range(boundary)
                                .nth(0)
                                .map(|v| *v.1)
                        })
                        .unwrap_or(false)
                    {
                        print_schema.generate_missing_sql_type_definitions = Some(false)
                    }

                    if let Some(except_rules) = &except_custom_type_definitions_with_indices {
                        if let Some(rules) = except_rules.range(boundary).nth(0) {
                            print_schema
                                .except_custom_type_definitions
                                .clone_from(rules.1);
                        }
                    }

                    let custom_type_derives = custom_type_derives_with_indices
                        .clone()
                        .map(|v| v.range(boundary).map(|v| v.1.clone()).collect())
                        .unwrap_or(vec![]);
                    if !custom_type_derives.is_empty() {
                        print_schema.custom_type_derives = Some(custom_type_derives);
                    }
                    if let Some(sqlite_integer_primary_key_is_bigint) =
                        sqlite_integer_primary_key_is_bigint_with_indices
                            .clone()
                            .and_then(|with_docs_with_indices| {
                                with_docs_with_indices.range(boundary).nth(0).map(|v| *v.1)
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
            if let Some(schema_name) = matches.get_one::<String>("schema") {
                config.schema = Some(schema_name.clone())
            }
            if matches.get_flag("with-docs") {
                config.with_docs = DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment;
            }
            if let Some(doc_config) = matches.get_one::<String>("with-docs-config") {
                config.with_docs = doc_config.parse().map_err(|_| {
                    crate::errors::Error::UnsupportedFeature(format!(
                        "Invalid documentation config mode: {doc_config}"
                    ))
                })?;
            }

            if let Some(sorting) = matches.get_one::<String>("column-sorting") {
                match sorting as &str {
                    "ordinal_position" => config.column_sorting = ColumnSorting::OrdinalPosition,
                    "name" => config.column_sorting = ColumnSorting::Name,
                    _ => {
                        return Err(crate::errors::Error::UnsupportedFeature(format!(
                            "Invalid column sorting mode: {sorting}"
                        )));
                    }
                }
            }

            if let Some(path) = matches.get_one::<PathBuf>("patch-file") {
                config.patch_file = Some(path.clone());
            }

            if let Some(types) = matches.get_many("import-types") {
                let types = types.cloned().collect();
                config.import_types = Some(types);
            }

            if let Some(except_rules) = matches.get_many("except-custom-type-definitions") {
                let regexes: Vec<String> = except_rules.cloned().collect();
                config.except_custom_type_definitions = regexes
                    .into_iter()
                    .map(|x| regex::Regex::new(&x).map(Into::into))
                    .collect::<Result<Vec<Regex>, _>>()?;
            }

            if matches.get_flag("generate-custom-type-definitions") {
                config.generate_missing_sql_type_definitions = Some(false);
            }

            if let Some(derives) = matches.get_many("custom-type-derives") {
                let derives = derives.cloned().collect();
                config.custom_type_derives = Some(derives);
            }
            if matches.get_flag("sqlite-integer-primary-key-is-bigint") {
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

    fn set_relative_path_base(&mut self, base: &Path) {
        if let Some(ref mut file) = self.file {
            if file.is_relative() {
                *file = base.join(&file);
            }
        }

        if let Some(ref mut patch_file) = self.patch_file {
            if patch_file.is_relative() {
                *patch_file = base.join(&patch_file);
            }
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

    pub fn set_filter(mut self, matches: &ArgMatches) -> Result<Self, crate::errors::Error> {
        let table_names = matches
            .get_many::<String>("table-name")
            .unwrap_or_default()
            .map(|table_name_regex| regex::Regex::new(table_name_regex).map(Into::into))
            .collect::<Result<Vec<Regex>, _>>()?;

        if matches
            .try_get_one::<bool>("only-tables")?
            .cloned()
            .unwrap_or(false)
        {
            self.filter = Filtering::OnlyTables(table_names)
        } else if matches
            .try_get_one::<bool>("except-tables")?
            .cloned()
            .unwrap_or(false)
        {
            self.filter = Filtering::ExceptTables(table_names)
        }
        Ok(self)
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

#[derive(Clone, Debug)]
pub enum Filtering {
    OnlyTables(Vec<Regex>),
    ExceptTables(Vec<Regex>),
    None,
}

#[allow(clippy::derivable_impls)] // that's not supported on rust 1.65
impl Default for Filtering {
    fn default() -> Self {
        Filtering::None
    }
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
