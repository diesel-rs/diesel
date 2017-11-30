// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "clippy",
           allow(option_map_unwrap_or_else, option_map_unwrap_or, match_same_arms,
                   type_complexity))]
#![cfg_attr(feature = "clippy",
           warn(option_unwrap_used, result_unwrap_used, print_stdout,
                  wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                  unicode_not_nfc, enum_glob_use, if_not_else, items_after_statements,
                  used_underscore_binding))]
#![cfg_attr(all(test, feature = "clippy"), allow(result_unwrap_used))]

#[macro_use]
extern crate diesel;

mod data_structures;
mod foreign_keys;
mod inference;
mod table_data;

#[cfg(feature = "uses_information_schema")]
mod information_schema;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use data_structures::*;
pub use foreign_keys::*;
pub use inference::*;
pub use table_data::*;
