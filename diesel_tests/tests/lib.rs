#![recursion_limit = "1024"]

// Running wasm tests on dedicated_worker
#[cfg(test)]
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);

#[macro_use]
extern crate assert_matches;

#[macro_use]
extern crate diesel;

mod aggregate_expressions;
mod alias;
#[cfg(not(feature = "sqlite"))]
mod annotations;
mod associations;
mod boxed_queries;
mod combination;
mod connection;
#[cfg(feature = "postgres")]
mod copy;
#[cfg(feature = "postgres")]
mod custom_types;
mod debug;
mod delete;
mod deserialization;
mod distinct;
mod errors;
mod expressions;
mod filter;
mod filter_operators;
mod find;
mod group_by;
mod having;
mod index;
mod insert;
mod insert_from_select;
mod instrumentation;
mod internal_details;
mod joins;
mod limit_offset;
mod macros;
#[cfg(feature = "postgres")]
mod only;
#[cfg(not(feature = "sqlite"))]
mod operations;
mod order;
mod perf_details;
#[cfg(feature = "postgres")]
mod query_fragment;
mod raw_sql;
mod schema;
mod schema_dsl;
mod schema_inference;
mod select;
mod select_by;
mod serialize_as;
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
mod transactions;
mod types;
mod types_roundtrip;
mod update;

/// Re-export it, because the `td::test` macro use `crate::init_sqlite`
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "sqlite"))]
pub use diesel::init_sqlite;
