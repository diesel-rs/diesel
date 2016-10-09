extern crate diesel;

use quote;
use syn;
use std::path::{Path, PathBuf};
use util::str_value_of_meta_item;
use diesel_codegen_shared::migrations::resolve_migrations_directory;

pub fn derive_embedded_migrations(input: syn::MacroInput) -> quote::Tokens {
    let options = get_options_from_input(&input.attrs);
    let dir = ::std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let callside = PathBuf::from(dir);
    let migration_path = get_option(options, "migration_path")
        .map(Path::new);
    let migration_dir = t!(resolve_migrations_directory(callside.as_path(),
        migration_path));
    let migrations = migration_literals_from_path(migration_dir.as_path());
    quote!(
        mod embedded_migrations {
            extern crate diesel;

            use self::diesel::migrations::*;
            use self::diesel::connection::SimpleConnection;
            
            const ALL_MIGRATIONS: &'static [&'static Migration] = &[ #(migrations),*];

            pub fn run<C: MigrationConnection>(conn: &C) -> Result<(), RunMigrationsError> {
                run_with_output(conn, &mut ::std::io::sink())
            }

            pub fn run_with_output<C: MigrationConnection>(conn: &C, out: &mut ::std::io::Write)
                                                           -> Result<(), RunMigrationsError>
            {
                run_migrations(conn, ALL_MIGRATIONS.iter().map(|v| *v), out)
            }
        }
    )
}


fn migration_literals_from_path(path: &Path) -> Vec<quote::Tokens> {
    use self::diesel::migrations::migration_paths_in_directory;

     t!(migration_paths_in_directory(&path)).into_iter()
        .map(|e| migration_literal_from_path(&e.path()))
            .collect()
}

fn migration_literal_from_path(path: &Path) -> quote::Tokens {
    use self::diesel::migrations::version_from_path;

    let version = t!(version_from_path(path));
    let sql_file = path.join("up.sql");
    let sql_file_path = sql_file.to_string_lossy();

    quote!(&EmbeddedMigration{
        version: #version,
        up_sql: include_str!(#sql_file_path),
    })
}

fn get_options_from_input(attrs: &[syn::Attribute]) -> Option<&[syn::MetaItem]> {
    let option = attrs.iter().find(|a| a.name() == "options").map(|a| &a.value);
    match option{
        Some(&syn::MetaItem::List(_, ref options)) => Some(options),
        _=> None
    }
}

fn get_option<'a>(
    options: Option<&'a [syn::MetaItem]>,
    option_name: &str,
) -> Option<&'a str> {
    match options.map(|o|{
        o.iter().find(|a| a.name() == option_name)
            .map(|a| str_value_of_meta_item(a, option_name))
    }) {
        Some(Some(r)) => Some(r),
        _ => None,
    }
}


