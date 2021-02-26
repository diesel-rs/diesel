use crate::migrations::migration_directory_from_given_path;
use migrations_internals::{migrations_directories, version_from_path, TomlMetadata};
use quote::quote;
use std::error::Error;
use std::fs::DirEntry;
use std::path::Path;

pub fn expand(path: String) -> proc_macro2::TokenStream {
    let migrations_path_opt = if path.is_empty() {
        None
    } else {
        Some(path.replace("\"", ""))
    };
    let migrations_expr =
        migration_directory_from_given_path(migrations_path_opt.as_ref().map(String::as_str))
            .unwrap();
    let embeded_migrations = migration_literals_from_path(&migrations_expr).unwrap();

    quote! {
        diesel_migrations::EmbededMigrations::new(&[#(#embeded_migrations,)*])
    }
}

fn migration_literals_from_path(
    path: &Path,
) -> Result<Vec<proc_macro2::TokenStream>, Box<dyn Error>> {
    let mut migrations = migrations_directories(path).collect::<Result<Vec<_>, _>>()?;

    migrations.sort_by_key(DirEntry::path);

    migrations
        .into_iter()
        .map(|e| migration_literal_from_path(&e.path()))
        .collect()
}

fn migration_literal_from_path(path: &Path) -> Result<proc_macro2::TokenStream, Box<dyn Error>> {
    let version: String = version_from_path(path).expect("Invalid version")?;
    let up_sql = path.join("up.sql");
    let up_sql_path = up_sql.to_str();
    let down_sql = path.join("up.sql");
    let down_sql_path = down_sql.to_str();
    let metadata = TomlMetadata::read_from_file(&path.join("metadata.toml")).unwrap_or_default();
    let run_in_transaction = metadata.run_in_transaction;

    Ok(quote!(diesel_migrations::EmbededMigration::new(
        include_str!(#up_sql_path),
        include_str!(#down_sql_path),
        #version,
        diesel_migrations::TomlMetadataWrapper::new(#run_in_transaction)
    )))
}
