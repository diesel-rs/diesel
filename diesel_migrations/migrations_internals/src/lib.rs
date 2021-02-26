//#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::option_map_unwrap_or_else,
    clippy::option_map_unwrap_or,
    clippy::match_same_arms,
    clippy::type_complexity
)]
#![warn(
    clippy::option_unwrap_used,
    clippy::result_unwrap_used,
    clippy::print_stdout,
    clippy::wrong_pub_self_convention,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::enum_glob_use,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]

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
        path.parent()
            .and_then(|p| search_for_migrations_directory(p))
        //.unwrap_or_else(|| Err(MigrationError::MigrationDirectoryNotFound(path.into())))
        //.map_err(|_| MigrationError::MigrationDirectoryNotFound(path.into()))
    }
}

pub fn valid_sql_migration_directory(path: &Path) -> bool {
    file_names(path)
        .map(|files| files.contains(&"down.sql".into()) && files.contains(&"up.sql".into()))
        .unwrap_or(false)
}

pub fn version_from_path(path: &Path) -> Option<Result<String, std::io::Error>> {
    path.file_name()
        .unwrap_or_else(|| panic!("Can't get file name from path `{:?}`", path))
        .to_string_lossy()
        .split('_')
        .next()
        .map(|s| Ok(s.replace('-', "")))
    //.unwrap_or_else(|| Err(MigrationError::UnknownMigrationFormat(path.to_path_buf())))
}

pub fn file_names(path: &Path) -> Result<Vec<String>, std::io::Error> {
    path.read_dir()?
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_name().to_string_lossy().starts_with('.') => None,
            Ok(entry) => Some(Ok(entry.file_name().to_string_lossy().into_owned())),
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()
    //.map_err(|e: std::io::Error| MigrationError::IoError(e, Some(path.to_path_buf())))
}

pub fn migrations_directories<'a>(
    path: &'a Path,
) -> impl Iterator<Item = Result<DirEntry, std::io::Error>> + 'a {
    //    let map_io_err = move |io_err| MigrationError::IoError(io_err, Some(path.to_path_buf()));

    path.read_dir()
        //.map_err(map_io_err)
        .into_iter()
        .flat_map(|read_dir| read_dir)
        .filter_map(move |entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => return Some(Err(e)),
            };
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => return Some(Err(e)),
            };
            if metadata.is_file() {
                return None;
            }
            if entry.file_name().to_string_lossy().starts_with('.') {
                None
            } else {
                Some(Ok(entry))
            }
        })
}
