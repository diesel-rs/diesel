// Built-in Lints
// Clippy lints
#![allow(
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::type_complexity
)]
#![warn(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::enum_glob_use,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding,
    missing_debug_implementations,
    missing_copy_implementations
)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{DirEntry, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[doc(hidden)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[allow(missing_copy_implementations)]
pub struct TomlMetadata {
    #[serde(default)]
    pub run_in_transaction: bool,

    #[serde(flatten)]
    additional_fields: HashMap<String, serde_json::Value>,
}

impl Default for TomlMetadata {
    fn default() -> Self {
        Self {
            run_in_transaction: true,
            additional_fields: HashMap::default(),
        }
    }
}

impl TomlMetadata {
    pub fn new(run_in_transaction: bool) -> Self {
        Self {
            run_in_transaction,
            additional_fields: HashMap::new(),
        }
    }

    pub fn read_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut toml = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut toml)?;
        Self::from_toml_str(&toml)
    }

    pub fn from_toml_str(toml: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(toml::from_str(toml)?)
    }

    pub fn to_toml_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(toml::to_string(&self)?)
    }

    pub fn serialized_additional_fields(&self) -> HashMap<String, String> {
        let mut map = HashMap::with_capacity(self.additional_fields.len());
        for (k, v) in self.additional_fields.iter() {
            map.insert(k.clone(), serde_json::to_string(v).unwrap_or_default());
        }
        map
    }
}

pub fn search_for_migrations_directory(path: &Path) -> Option<PathBuf> {
    let migration_path = path.join("migrations");
    if migration_path.is_dir() {
        Some(migration_path)
    } else {
        path.parent().and_then(search_for_migrations_directory)
    }
}

pub fn valid_sql_migration_directory(path: &Path) -> bool {
    file_names(path).map_or(false, |files| files.iter().any(|f| f == "up.sql"))
}

pub fn version_from_string(path: &str) -> Option<String> {
    path.split('_').next().map(|s| s.replace('-', ""))
}

fn file_names(path: &Path) -> Result<Vec<OsString>, std::io::Error> {
    path.read_dir()?
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_name().to_string_lossy().starts_with('.') => None,
            Ok(entry) => Some(Ok(entry.file_name())),
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()
}

pub fn migrations_directories(
    path: &'_ Path,
) -> Result<impl Iterator<Item = Result<DirEntry, std::io::Error>> + '_, std::io::Error> {
    Ok(path.read_dir()?.into_iter().filter_map(|entry_res| {
        entry_res
            .and_then(|entry| {
                Ok(
                    if entry.metadata()?.is_file()
                        || entry.file_name().to_string_lossy().starts_with('.')
                    {
                        None
                    } else {
                        Some(entry)
                    },
                )
            })
            .transpose()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    static RAW_TOML: &str = r#"
run_in_transaction = false
boolean_field = false
string_field = "Something"

[foo]
name = "Bar"
"#;

    #[test]
    fn extracts_additional_fields() {
        let md = TomlMetadata::from_toml_str(RAW_TOML).unwrap();
        dbg!(&md);
        assert!(!md.run_in_transaction);
        assert!(md.additional_fields.contains_key("boolean_field"));
        assert!(md.additional_fields.contains_key("string_field"));
        assert!(md.additional_fields.contains_key("foo"));
        assert!(!md.additional_fields.contains_key("name"));
    }

    #[test]
    fn round_trip() {
        let md = TomlMetadata::from_toml_str(RAW_TOML).unwrap();
        let toml = md.to_toml_string().unwrap();
        let new = TomlMetadata::from_toml_str(&toml).unwrap();
        assert_eq!(md.run_in_transaction, new.run_in_transaction);
        for (k, v) in md.additional_fields.iter() {
            assert_eq!(v, new.additional_fields.get(k).unwrap());
        }
    }

    #[test]
    fn additional_fields_serialization() {
        let md = TomlMetadata::from_toml_str(RAW_TOML).unwrap();
        let fields = md.serialized_additional_fields();
        assert_eq!("\"Something\"", fields.get("string_field").unwrap());
        assert_eq!("false", fields.get("boolean_field").unwrap());
        assert_eq!(r#"{"name":"Bar"}"#, fields.get("foo").unwrap());
    }
}
