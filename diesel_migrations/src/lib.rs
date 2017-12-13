// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(option_map_unwrap_or_else, option_map_unwrap_or))]
#![cfg_attr(feature = "clippy",
            warn(wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, if_not_else, items_after_statements, used_underscore_binding))]

extern crate migrations_internals;
#[cfg_attr(feature = "clippy", allow(useless_attribute))]
#[allow(unused_imports)]
#[macro_use]
extern crate migrations_macros;
#[doc(hidden)]
pub use migrations_macros::*;
#[doc(inline)]
pub use migrations_internals::*;

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
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_migrations;
/// # include!("../../diesel/src/doctest_setup.rs");
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
