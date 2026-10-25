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
    file_names(path).is_ok_and(|files| files.iter().any(|f| f == "up.sql"))
}

pub fn version_from_string(path: &str) -> Option<String> {
    path.split('_').next().map(|s| s.replace('-', ""))
}

/// Matches the `VARCHAR(50)` version column in `diesel/src/migration/setup_migration_table.sql`.
///
/// Full-name versions are truncated before execution so the history insert fits on every backend.
pub const MAX_MIGRATION_VERSION_LENGTH: usize = 50;

/// Builds a version from the full migration name, used when `include_name_in_version` is enabled.
///
/// Dashes and underscores are removed, and the result is truncated to
/// [`MAX_MIGRATION_VERSION_LENGTH`] characters.
pub fn full_name_version(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '-' | '_'))
        .take(MAX_MIGRATION_VERSION_LENGTH)
        .collect()
}

/// The part of `diesel.toml` that affects migration versions. Other keys are ignored.
#[derive(Default, serde::Deserialize)]
struct DieselToml {
    #[serde(default)]
    migrations_directory: MigrationsDirectory,
}

#[derive(Default, serde::Deserialize)]
struct MigrationsDirectory {
    #[serde(default)]
    include_name_in_version: bool,
}

/// Reads `include_name_in_version` from the `[migrations_directory]` table of a `diesel.toml`.
pub fn include_name_in_version(diesel_toml: &str) -> Result<bool, toml::de::Error> {
    let config: DieselToml = toml::from_str(diesel_toml)?;
    Ok(config.migrations_directory.include_name_in_version)
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
                    } else if entry.metadata()?.is_dir() {
                        let inner_dir_entry_count = entry.path().read_dir()?.count();

                        if inner_dir_entry_count == 0 {
                            None
                        } else {
                            Some(entry)
                        }
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

    #[test]
    fn full_name_version_removes_dashes_and_underscores() {
        assert_eq!(
            full_name_version("2026-10-25-193442-0000_hello-world"),
            "202610251934420000helloworld"
        );
    }

    #[test]
    fn full_name_version_truncates_to_max_characters() {
        let name = format!("2026-10-25-193442-0000_{}", "\u{e9}".repeat(60));
        let version = full_name_version(&name);
        assert_eq!(version.chars().count(), MAX_MIGRATION_VERSION_LENGTH);
        assert_eq!(
            version,
            format!("202610251934420000{}", "\u{e9}".repeat(32))
        );
    }

    #[test]
    fn include_name_in_version_defaults_to_false() {
        assert_eq!(include_name_in_version(""), Ok(false));
        assert_eq!(
            include_name_in_version("[print_schema]\nfile = \"src/schema.rs\"\n"),
            Ok(false)
        );
        assert_eq!(
            include_name_in_version("[migrations_directory]\ndir = \"migrations\"\n"),
            Ok(false)
        );
    }

    #[test]
    fn include_name_in_version_reads_the_setting() {
        assert_eq!(
            include_name_in_version(
                "[migrations_directory]\ndir = \"migrations\"\ninclude_name_in_version = true\n"
            ),
            Ok(true)
        );
        assert!(
            include_name_in_version("[migrations_directory]\ninclude_name_in_version = 1\n")
                .is_err()
        );
    }
}
