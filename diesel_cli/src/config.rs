use clap::ArgMatches;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_regex::Serde as RegexWrapper;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::{env, fmt};

use super::find_project_root;
use crate::infer_schema_internals::TableName;
use crate::print_schema;
use crate::print_schema::ColumnSorting;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub print_schema: PrintSchema,
    #[serde(default)]
    pub migrations_directory: Option<MigrationsDirectory>,
}

impl Config {
    pub fn file_path(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("CONFIG_FILE")
            .cloned()
            .or_else(|| env::var_os("DIESEL_CONFIG_FILE").map(PathBuf::from))
            .unwrap_or_else(|| find_project_root().unwrap_or_default().join("diesel.toml"))
    }

    pub fn read(matches: &ArgMatches) -> Result<Self, Box<dyn Error + Send + Sync + 'static>> {
        let path = Self::file_path(matches);

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let mut result = toml::from_str::<Self>(&content)?;
            result.set_relative_path_base(path.parent().unwrap());
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
}

#[derive(Default, Deserialize)]
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
    pub custom_type_derives: Option<Vec<String>>,
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
}

#[derive(Default, Deserialize)]
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
                            ))
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
