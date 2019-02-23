use proc_macro2;
use syn;

use migrations::migration_directory_from_given_path;
use migrations_internals::{migration_paths_in_directory, version_from_path};
use std::error::Error;
use std::path::Path;

use util::{get_option, get_options_from_input};

pub fn derive_embed_migrations(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    fn bug() -> ! {
        panic!(
            "This is a bug. Please open a Github issue \
             with your invocation of `embed_migrations!"
        );
    }

    let options =
        get_options_from_input(&parse_quote!(embed_migrations_options), &input.attrs, bug);
    let migrations_path_opt = options
        .as_ref()
        .map(|o| get_option(o, "migrations_path", bug));
    let migrations_expr =
        migration_directory_from_given_path(migrations_path_opt.as_ref().map(String::as_str))
            .and_then(|path| migration_literals_from_path(&path));
    let migrations_expr = match migrations_expr {
        Ok(v) => v,
        Err(e) => panic!("Error reading migrations: {}", e),
    };

    // These are split into multiple `quote!` calls to avoid recursion limit
    let embedded_migration_def = quote!(
        struct EmbeddedMigration {
            version: &'static str,
            up_sql: &'static str,
        }

        impl Migration for EmbeddedMigration {
            fn version(&self) -> &str {
                self.version
            }

            fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
                conn.batch_execute(self.up_sql).map_err(Into::into)
            }

            fn revert(&self, _conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
                unreachable!()
            }
        }
    );

    let run_fns = quote!(
        pub fn run<C: MigrationConnection>(conn: &C) -> Result<(), RunMigrationsError> {
            run_with_output(conn, &mut io::sink())
        }

        pub fn run_with_output<C: MigrationConnection>(
            conn: &C,
            out: &mut io::Write,
        ) -> Result<(), RunMigrationsError> {
            run_migrations(conn, ALL_MIGRATIONS.iter().map(|v| *v), out)
        }
    );

    quote! {
        extern crate diesel;
        extern crate diesel_migrations;

        use self::diesel_migrations::*;
        use self::diesel::connection::SimpleConnection;
        use std::io;

        const ALL_MIGRATIONS: &[&Migration] = &[#(#migrations_expr),*];

        #embedded_migration_def

        #run_fns
    }
}

fn migration_literals_from_path(path: &Path) -> Result<Vec<proc_macro2::TokenStream>, Box<Error>> {
    let mut migrations = migration_paths_in_directory(path)?;

    migrations.sort_by_key(|a| a.path());

    migrations
        .into_iter()
        .map(|e| migration_literal_from_path(&e.path()))
        .collect()
}

fn migration_literal_from_path(path: &Path) -> Result<proc_macro2::TokenStream, Box<Error>> {
    let version = version_from_path(path)?;
    let sql_file = path.join("up.sql");
    let sql_file_path = sql_file.to_str();

    Ok(quote!(&EmbeddedMigration {
        version: #version,
        up_sql: include_str!(#sql_file_path),
    }))
}
