use crate::migrations::{diesel_config_file, migration_directory_from_given_path};
use migrations_internals::{
    TomlMetadata, full_name_version, include_name_in_version, migrations_directories,
    version_from_string,
};
use quote::quote;
use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, DirEntry};
use std::path::Path;

pub fn expand(path: Option<syn::LitStr>) -> proc_macro2::TokenStream {
    let migrations_path_opt = path.map(|l| l.value());
    let migrations_expr = migration_directory_from_given_path(migrations_path_opt.as_deref())
        .unwrap_or_else(|_| {
            panic!("Failed to receive migrations dir from {migrations_path_opt:?}")
        });
    let config_file = diesel_config_file().expect("Failed to search for diesel.toml");
    let include_name = config_file.as_deref().is_some_and(|config_file| {
        read_include_name_in_version(config_file)
            .unwrap_or_else(|e| panic!("Failed to read `{}`: {e}", config_file.display()))
    });
    let embedded_migrations = migration_literals_from_path(&migrations_expr, include_name)
        .expect("Failed to read migration literals");

    let migrations = quote! {
        diesel_migrations::EmbeddedMigrations::new(&[#(#embedded_migrations,)*])
    };
    match config_file {
        // Including the file makes cargo rebuild the crate when `diesel.toml` changes
        Some(config_file) => {
            let config_file = config_file.to_str();
            quote! {{
                const _: &[u8] = include_bytes!(#config_file);
                #migrations
            }}
        }
        None => migrations,
    }
}

fn read_include_name_in_version(config_file: &Path) -> Result<bool, Box<dyn Error>> {
    Ok(include_name_in_version(&fs::read_to_string(config_file)?)?)
}

fn migration_literals_from_path(
    path: &Path,
    include_name_in_version: bool,
) -> Result<Vec<proc_macro2::TokenStream>, Box<dyn Error>> {
    let mut migrations = migrations_directories(path)?.collect::<Result<Vec<_>, _>>()?;

    migrations.sort_by_key(DirEntry::path);

    let mut seen_versions = HashSet::new();
    Ok(migrations
        .into_iter()
        .map(|e| {
            let version = include_name_in_version.then(|| {
                let version = full_name_version(&e.file_name().to_string_lossy());
                if !seen_versions.insert(version.clone()) {
                    panic!(
                        "Migration version {version} is shared by multiple migrations. \
                         Rename the migrations so their normalized, truncated versions are unique."
                    );
                }
                version
            });
            migration_literal_from_path(&e.path(), version.as_deref())
        })
        .collect())
}

fn migration_literal_from_path(path: &Path, version: Option<&str>) -> proc_macro2::TokenStream {
    let name = path
        .file_name()
        .unwrap_or_else(|| panic!("Can't get file name from path `{path:?}`"))
        .to_string_lossy();
    if version_from_string(&name).is_none() {
        panic!(
            "Invalid migration directory: the directory's name should be \
             <timestamp>_<name_of_migration>, and it should contain \
             up.sql and optionally down.sql."
        );
    }
    let up_sql_path = path.join("up.sql");
    let up_sql_path = up_sql_path.to_str();
    let down_sql_path = path.join("down.sql");
    let metadata = TomlMetadata::read_from_file(&path.join("metadata.toml")).unwrap_or_default();
    let run_in_transaction = metadata.run_in_transaction;

    let down_sql = match down_sql_path.metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => quote! { None },
        _ => {
            let down_sql_path = down_sql_path.to_str();
            quote! { Some(include_str!(#down_sql_path)) }
        }
    };

    let name = match version {
        Some(version) => quote! { diesel_migrations::EmbeddedName::with_version(#name, #version) },
        None => quote! { diesel_migrations::EmbeddedName::new(#name) },
    };

    quote!(diesel_migrations::EmbeddedMigration::new(
        include_str!(#up_sql_path),
        #down_sql,
        #name,
        diesel_migrations::TomlMetadataWrapper::new(#run_in_transaction)
    ))
}

#[cfg(test)]
mod tests {
    use super::migration_literals_from_path;
    use std::fs;
    use std::path::Path;
    use tempfile::Builder;

    fn create_migration(migrations: &Path, name: &str) {
        let migration = migrations.join(name);
        fs::create_dir_all(&migration).unwrap();
        fs::write(migration.join("up.sql"), "SELECT 1;").unwrap();
    }

    #[test]
    fn migrations_keep_timestamp_versions_by_default() {
        let tempdir = Builder::new().prefix("diesel").tempdir().unwrap();
        create_migration(tempdir.path(), "2026-10-25-193442-0000_hello_world");

        let literals = migration_literals_from_path(tempdir.path(), false).unwrap();
        let literal = literals[0].to_string();
        assert!(literal.contains("EmbeddedName :: new (\"2026-10-25-193442-0000_hello_world\")"));
    }

    #[test]
    fn migrations_embed_full_name_versions_when_enabled() {
        let tempdir = Builder::new().prefix("diesel").tempdir().unwrap();
        create_migration(tempdir.path(), "2026-10-25-193442-0000_hello_world");

        let literals = migration_literals_from_path(tempdir.path(), true).unwrap();
        let literal = literals[0].to_string();
        assert!(literal.contains(
            "EmbeddedName :: with_version (\"2026-10-25-193442-0000_hello_world\" , \"202610251934420000helloworld\")"
        ));
    }

    #[test]
    #[should_panic(expected = "shared by multiple migrations")]
    fn colliding_full_name_versions_fail_to_compile() {
        let tempdir = Builder::new().prefix("diesel").tempdir().unwrap();
        create_migration(tempdir.path(), "2026-10-25-193442-0000_hello_world");
        create_migration(tempdir.path(), "2026-10-25-193442-0000-hello-world");

        let _ = migration_literals_from_path(tempdir.path(), true);
    }
}
