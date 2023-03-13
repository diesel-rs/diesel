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

use std::ffi::OsString;
use std::fs::{DirEntry, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[doc(hidden)]
#[derive(Debug, serde::Deserialize)]
#[allow(missing_copy_implementations)]
pub struct TomlMetadata {
    #[serde(default)]
    pub run_in_transaction: bool,
}

impl Default for TomlMetadata {
    fn default() -> Self {
        Self {
            run_in_transaction: true,
        }
    }
}

impl TomlMetadata {
    pub const fn new(run_in_transaction: bool) -> Self {
        Self { run_in_transaction }
    }

    pub fn read_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut toml = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut toml)?;

        Ok(toml::from_str(&toml)?)
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
    Ok(path.read_dir()?.filter_map(|entry_res| {
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
