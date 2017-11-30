// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(option_map_unwrap_or_else, option_map_unwrap_or))]
#![cfg_attr(feature = "clippy",
            warn(wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, if_not_else, items_after_statements, used_underscore_binding))]

#[cfg_attr(feature = "clippy", allow(useless_attribute))]
#[allow(unused_imports)]
#[macro_use]
extern crate infer_schema_macros;
#[doc(hidden)]
pub use infer_schema_macros::*;

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
macro_rules! infer_table_from_schema {
    ($database_url: expr, $table_name: expr) => {
        #[derive(InferTableFromSchema)]
        #[infer_table_from_schema_options(database_url=$database_url, table_name=$table_name)]
        struct __DieselInferTableFromSchema;
    }
}
