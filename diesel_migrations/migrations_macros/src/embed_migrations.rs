use proc_macro2;
use syn;

use migrations::migration_directory_from_given_path;
use migrations_internals::{migration_paths_in_directory, version_from_path};
use std::error::Error;
use std::fs::DirEntry;
use std::path::Path;

use util::{get_option, get_options_from_input};

#[derive(Copy, Clone)]
enum Direction {
    Up,
    Down,
}

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

    let up_migrations_expr =
        migration_directory_from_given_path(migrations_path_opt.as_ref().map(String::as_str))
            .and_then(|path| migration_literals_from_path(&path, Direction::Up));
    let up_migrations_expr = match up_migrations_expr {
        Ok(v) => v,
        Err(e) => panic!("Error reading migrations: {}", e),
    };

    let down_migrations_expr =
        migration_directory_from_given_path(migrations_path_opt.as_ref().map(String::as_str))
            .and_then(|path| migration_literals_from_path(&path, Direction::Down));
    let down_migrations_expr = match down_migrations_expr {
        Ok(v) => v,
        Err(e) => panic!("Error reading migrations: {}", e),
    };

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
            run_migrations(conn, UP_MIGRATIONS.iter().map(|v| *v), out)
        }

        pub fn revert<C: MigrationConnection>(conn: &C) -> Result<(), RunMigrationsError> {
            run_migrations(conn, DOWN_MIGRATIONS.iter().map(|v| *v), &mut io::sink())
        }
    );

    quote! {
        extern crate diesel;
        extern crate diesel_migrations;

        use self::diesel_migrations::*;
        use self::diesel::connection::SimpleConnection;
        use std::io;

        const UP_MIGRATIONS: &[&Migration] = &[#(#up_migrations_expr),*];
        const DOWN_MIGRATIONS: &[&Migration] = &[#(#down_migrations_expr),*];

        #embedded_migration_def

        #run_fns
    }
}

fn migration_literals_from_path(
    path: &Path,
    direction: Direction,
) -> Result<Vec<proc_macro2::TokenStream>, Box<dyn Error>> {
    let mut migrations = migration_paths_in_directory(path)?;

    migrations.sort_by_key(DirEntry::path);

    match direction {
        Direction::Up => (),
        Direction::Down => migrations.reverse(),
    }

    migrations
        .into_iter()
        .map(|e| migration_literal_from_path(&e.path(), direction))
        .collect()
}

fn migration_literal_from_path(
    path: &Path,
    direction: Direction,
) -> Result<proc_macro2::TokenStream, Box<dyn Error>> {
    let version = version_from_path(path)?;
    let sql_file = path.join(match direction {
        Direction::Up => "up.sql",
        Direction::Down => "down.sql",
    });
    let up_sql_file_path = sql_file.to_str();
    let down_sql_file_path = up_sql_file_path;

    Ok(quote!(&EmbeddedMigration {
        version: #version,
        up_sql: include_str!(#up_sql_file_path),
        down_sql: include_str!(#down_sql_file_path),
    }))
}
