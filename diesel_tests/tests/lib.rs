#![cfg_attr(feature = "unstable", feature(proc_macro))]

extern crate quickcheck;
#[macro_use] extern crate assert_matches;
#[macro_use] extern crate diesel;
#[cfg(feature = "unstable")]
#[macro_use] extern crate diesel_codegen;
extern crate dotenv;

#[cfg(feature = "unstable")]
include!("lib.in.rs");

#[cfg(not(feature = "unstable"))]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

mod boxed_queries;
mod connection;
// This should be in lib.in.rs restricted to PG, but
// syntex compiles the file even if the feature is unset,
// and the macro call is invalid on SQLite.
#[cfg(all(feature = "unstable", feature = "postgres"))]
mod custom_schemas;
mod debug;
mod delete;
mod errors;
mod expressions;
mod filter;
mod filter_operators;
mod find;
mod group_by;
mod internal_details;
mod joins;
mod macros;
mod order;
mod perf_details;
mod schema_dsl;
mod select;
mod transactions;
mod types;
mod types_roundtrip;
