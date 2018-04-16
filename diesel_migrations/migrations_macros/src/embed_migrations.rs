use syn;
use quote;

use migrations_internals::{migration_paths_in_directory, version_from_path};
use migrations::migration_directory_from_given_path;
use std::error::Error;
use std::path::Path;

use util::{get_option, get_options_from_input};

pub fn derive_embed_migrations(input: &syn::DeriveInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!(
            "This is a bug. Please open a Github issue \
             with your invocation of `embed_migrations!"
        );
    }

    let options = get_options_from_input("embed_migrations_options", &input.attrs, bug);
    let migrations_path_opt = options
        .as_ref()
        .map(|o| get_option(o, "migrations_path", bug));
    let migrations_expr = migration_directory_from_given_path(migrations_path_opt)
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

        pub fn run_with_output<C: MigrationConnection>(conn: &C, out: &mut io::Write)
            -> Result<(), RunMigrationsError>
        {
            run_migrations(conn, ALL_MIGRATIONS.iter().map(|v| *v), out)
        }
    );

    quote! {
        extern crate diesel;
        extern crate diesel_migrations;

        use self::diesel_migrations::*;
        use self::diesel::migration::*;
        use self::diesel::connection::SimpleConnection;
        use std::io;

        const ALL_MIGRATIONS: &[&Migration] = &[#(#migrations_expr),*];

        #embedded_migration_def

        #run_fns
    }
}

fn migration_literals_from_path(path: &Path) -> Result<Vec<quote::Tokens>, Box<Error>> {
    try!(migration_paths_in_directory(path))
        .into_iter()
        .map(|e| migration_literal_from_path(&e.path()))
        .collect()
}

fn migration_literal_from_path(path: &Path) -> Result<quote::Tokens, Box<Error>> {
    let version = try!(version_from_path(path));
    let sql_file = path.join("up.sql");
    let sql_file_path = sql_file.to_str();

    Ok(quote!(&EmbeddedMigration {
        version: #version,
        up_sql: include_str!(#sql_file_path),
    }))
}
