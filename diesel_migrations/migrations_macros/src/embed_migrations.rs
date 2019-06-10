use proc_macro2;
use syn;

use migrations::migration_directory_from_given_path;
use migrations_internals::{
    migration_paths_in_directory, version_from_path, TomlDatetime, TomlMetadata, TomlTable,
    TomlValue,
};
use std::error::Error;
use std::fs::{DirEntry, File};
use std::io::Read;
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
            metadata: Option<TomlMetadata>
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

            fn metadata(&self) -> Option<&dyn Metadata> {
                self.metadata.as_ref().map(|m| m as _)
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

fn migration_literals_from_path(
    path: &Path,
) -> Result<Vec<proc_macro2::TokenStream>, Box<dyn Error>> {
    let mut migrations = migration_paths_in_directory(path)?;

    migrations.sort_by_key(DirEntry::path);

    migrations
        .into_iter()
        .map(|e| migration_literal_from_path(&e.path()))
        .collect()
}

fn migration_literal_from_path(path: &Path) -> Result<proc_macro2::TokenStream, Box<dyn Error>> {
    let version = version_from_path(path)?;
    let sql_file = path.join("up.sql");
    let sql_file_path = sql_file.to_str();

    let metadata_file = path.join("metadata.toml");
    let metadata = if metadata_file.exists() {
        let metadata = migration_metadata_from_path(&metadata_file)?;
        quote!(Some(#metadata))
    } else {
        quote!(None)
    };

    Ok(quote!(&EmbeddedMigration {
        version: #version,
        up_sql: include_str!(#sql_file_path),
        metadata: #metadata
    }))
}

fn migration_metadata_from_path(path: &Path) -> Result<proc_macro2::TokenStream, Box<dyn Error>> {
    let mut buf = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    let value = TomlMetadata::from_slice(&buf)?.0;
    Ok(migration_metadata_from_value(&value))
}

fn migration_metadata_from_value(value: &TomlValue) -> proc_macro2::TokenStream {
    use migrations_internals::TomlValue::*;

    match value {
        String(s) => quote!(TomlValue::String(#s)),
        Integer(i) => quote!(TomlValue::Integer(#i)),
        Float(f) => quote!(TomlValue::Float(#f)),
        Boolean(b) => quote!(TomlValue::Boolean(#b)),
        Datetime(d) => {
            let datetime = migration_metadata_from_datetime(d);
            quote!(TomlValue::Datetime(#datetime))
        }
        Array(a) => {
            let array = a.iter().map(|v| migration_metadata_from_value(v));
            quote!(TomlValue::Array(vec![#(#array),*]))
        }
        Table(t) => {
            let table = migration_metadata_from_table(t);
            quote!(TomlValue::Table(#table))
        }
    }
}

fn migration_metadata_from_datetime(dt: &TomlDatetime) -> proc_macro2::TokenStream {
    // Unfortunately, the toml::Datetime type does not allow us to access the underlying data. We
    // are currently forced to serialize the datetime to a string and parse it again later.
    let dt_str = format!("{}", dt);
    quote!(TomlDatetime::from_str(#dt_str).expect("A previously valid toml::Datetime became invalid"))
}

fn migration_metadata_from_table(tbl: &TomlTable) -> proc_macro2::TokenStream {
    // toml uses a BTreeMap by default to represent a table
    let entries = tbl.iter().map(|(k, v)| {
        let value = migration_metadata_from_value(v);
        quote!(m.insert(String::from(#k), #value);)
    });
    quote! {
        {
            let mut m = TomlTable::new();
            #(#entries)*
            m
        }
    }
}
