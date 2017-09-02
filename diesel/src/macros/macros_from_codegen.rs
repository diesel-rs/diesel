#[macro_export]
/// Queries the database for the names of all tables, and calls
/// [`infer_table_from_schema!`](macro.infer_table_from_schema.html) for each
/// one. A schema name can optionally be passed to load from schemas other than
/// the default. If a schema name is given, the inferred tables will be wrapped
/// in a module with the same name.
///
/// Attempting to use the `env!` or `dotenv!` macros here will not work due to
/// limitations of the Macros 1.1 system, but you can pass a string in the form
/// `"env:SOME_ENV_VAR"` or `"dotenv:SOME_ENV_VAR"` to achieve the same effect.
///
/// This macro can only be used in combination with the `diesel_codegen` crate.
/// It will not work on its own.
///
/// If any column name would collide with a rust keyword, a `_` will
/// automatically be placed at the end of the name. For example, a column called
/// `type` will be referenced as `type_` in the generated module.
macro_rules! infer_schema {
    ($database_url: expr) => {
        mod __diesel_infer_schema {
            #[derive(InferSchema)]
            #[infer_schema_options(database_url=$database_url)]
            struct _Dummy;
        }
        pub use self::__diesel_infer_schema::*;
    };

    ($database_url: expr, $schema_name: expr) => {
        mod __diesel_infer_schema {
            #[derive(InferSchema)]
            #[infer_schema_options(database_url=$database_url, schema_name=$schema_name)]
            struct _Dummy;
        }
        pub use self::__diesel_infer_schema::*;
    };
}

#[macro_export]
/// Establishes a database connection at compile time, loads the schema
/// information about a table's columns, and invokes
/// [`table!`](macro.table.html) for you automatically. For tables in a schema
/// other than the default, the table name should be given as
/// `"schema_name.table_name"`.
///
/// Attempting to use the `env!` or `dotenv!` macros here will not work due to
/// limitations of the Macros 1.1 system, but you can pass a string in the form
/// `"env:SOME_ENV_VAR"` or `"dotenv:SOME_ENV_VAR"` to achieve the same effect.
///
/// At this time, the schema inference macros do not support types from third
/// party crates, and having any columns with a type not supported by the diesel
/// core crate will result in a compiler error (please [open an
/// issue](https://github.com/diesel-rs/diesel/issues/new) if this happens
/// unexpectedly for a type listed in our docs.)
///
/// This macro can only be used in combination with the `diesel_codegen` or
/// `diesel_codegen_syntex` crates. It will not work on its own.
macro_rules! infer_table_from_schema {
    ($database_url: expr, $table_name: expr) => {
        #[derive(InferTableFromSchema)]
        #[infer_table_from_schema_options(database_url=$database_url, table_name=$table_name)]
        struct __DieselInferTableFromSchema;
    }
}

#[macro_export]
/// This macro will read your migrations at compile time, and embed a module you can use to execute
/// them at runtime without the migration files being present on the file system. This is useful if
/// you would like to use Diesel's migration infrastructure, but want to ship a single executable
/// file (such as for embedded applications). It can also be used to apply migrations to an in
/// memory database (Diesel does this for its own test suite).
///
/// You can optionally pass the path to the migrations directory to this macro. When left
/// unspecified, Diesel Codegen will search for the migrations directory in the same way that
/// Diesel CLI does. If specified, the path should be relative to the directory where `Cargo.toml`
/// resides.
///
/// This macro can only be used in combination with the `diesel_codegen` or
/// `diesel_codegen_syntex` crates. It will not work on its own.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// # include!("../doctest_setup.rs");
/// # table! {
/// #   users {
/// #       id -> Integer,
/// #       name -> VarChar,
/// #   }
/// # }
/// #
/// # #[cfg(feature = "postgres")]
/// # embed_migrations!("../migrations/postgresql");
/// # #[cfg(all(feature = "mysql", not(feature = "postgres")))]
/// # embed_migrations!("../migrations/mysql");
/// # #[cfg(all(feature = "sqlite", not(any(feature = "postgres", feature = "mysql"))))]
/// embed_migrations!("../migrations/sqlite");
///
/// fn main() {
///     let connection = establish_connection();
///
///     // This will run the necessary migrations.
///     embedded_migrations::run(&connection);
///
///     // By default the output is thrown out. If you want to redirect it to stdout, you
///     // should call embedded_migrations::run_with_output.
///     embedded_migrations::run_with_output(&connection, &mut std::io::stdout());
/// }
/// ```
macro_rules! embed_migrations {
    () => {
        #[allow(dead_code)]
        mod embedded_migrations {
            #[derive(EmbedMigrations)]
            struct _Dummy;
        }
    };

    ($migrations_path: expr) => {
        #[allow(dead_code)]
        mod embedded_migrations {
            #[derive(EmbedMigrations)]
            #[embed_migrations_options(migrations_path=$migrations_path)]
            struct _Dummy;
        }
    }
}
