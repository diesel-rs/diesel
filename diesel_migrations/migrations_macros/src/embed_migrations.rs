use proc_macro2;
use syn;

use migrations::migration_directory_from_given_path;
use migrations_internals::{migration_paths_in_directory, version_from_path};
use std::error::Error;
use std::fs::DirEntry;
use std::path::Path;

use util::{get_option, get_options_from_input, get_rust_migrations_from_input};

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
    let rust_migrations_expr =
        get_rust_migrations_from_input(&parse_quote!(embed_rust_migrations), &input.attrs, bug)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(i, expr_str)| {
                let expr = syn::parse_str::<syn::Expr>(&expr_str).unwrap_or_else(|e| {
                    panic!("Migration string [{}] must contain an expression: {}", i, e)
                });
                quote!(Box::new(#expr))
            })
            .collect::<Vec<_>>();

    // These are split into multiple `quote!` calls to avoid recursion limit
    let embedded_migration_def = quote!(
        struct EmbeddedMigration {
            version: &'static str,
            up_sql: &'static str,
            down_sql: &'static str,
        }

        impl Migration for EmbeddedMigration {
            fn version(&self) -> &str {
                self.version
            }

            fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
                conn.batch_execute(self.up_sql).map_err(Into::into)
            }

            fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
                conn.batch_execute(self.down_sql).map_err(Into::into)
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
            run_migrations(conn, ALL_MIGRATIONS.iter().map(|v| &**v), out)
        }

        pub fn migration_with_version(ver: &str) -> Result<&'static Migration, MigrationError> {
            let migration = ALL_MIGRATIONS
                .iter()
                .map(|v| &**v)
                .find(|m| m.version() == ver);
            match migration {
                Some(m) => Ok(m),
                None => Err(MigrationError::UnknownMigrationVersion(ver.into())),
            }
        }

        pub fn revert_migration_with_version<Conn: Connection>(
            conn: &Conn,
            ver: &str,
            out: &mut io::Write,
        ) -> Result<(), RunMigrationsError> {
            migration_with_version(ver)
                .map_err(Into::into)
                .and_then(|m| revert_migration(conn, &m, out))
        }

        pub fn revert_latest_embedded_migration<Conn>(
            conn: &Conn,
            out: &mut io::Write,
        ) -> Result<String, RunMigrationsError>
        where
            Conn: MigrationConnection,
        {
            setup_database(conn)?;
            let latest_migration_version =
                conn.latest_run_migration_version()?.ok_or_else(|| {
                    RunMigrationsError::MigrationError(MigrationError::NoMigrationRun)
                })?;
            revert_migration_with_version(conn, &latest_migration_version, out)
                .map(|_| latest_migration_version)
        }
    );

    quote! {
        extern crate diesel;
        extern crate diesel_migrations;

        use self::diesel_migrations::*;
        use self::diesel::connection::{Connection, SimpleConnection};
        use std::io;

        lazy_static! {
            pub static ref ALL_MIGRATIONS: Vec<Box<Migration + Send + Sync>> = {
                let mut migrations: Vec<Box<Migration + Send + Sync>> = vec![
                    #(#migrations_expr,)*
                    #(#rust_migrations_expr,)*
                ];
                migrations.sort_by(|a, b| a.version().cmp(b.version()));
                migrations
            };
        }

        #embedded_migration_def

        #run_fns
    }
}

fn migration_literals_from_path(path: &Path) -> Result<Vec<proc_macro2::TokenStream>, Box<Error>> {
    let mut migrations = migration_paths_in_directory(path)?;

    migrations.sort_by_key(DirEntry::path);

    migrations
        .into_iter()
        .map(|e| migration_literal_from_path(&e.path()))
        .collect()
}

fn migration_literal_from_path(path: &Path) -> Result<proc_macro2::TokenStream, Box<Error>> {
    let version = version_from_path(path)?;
    let sql_file_up = path.join("up.sql");
    let sql_file_path_up = sql_file_up.to_str();
    let sql_file_down = path.join("down.sql");
    let sql_file_path_down = sql_file_down.to_str();

    Ok(quote!(Box::new(EmbeddedMigration {
        version: #version,
        up_sql: include_str!(#sql_file_path_up),
        down_sql: include_str!(#sql_file_path_down),
    })))
}
