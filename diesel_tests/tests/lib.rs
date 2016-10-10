#![cfg_attr(feature = "unstable", feature(plugin, rustc_macro))]
#![cfg_attr(feature = "unstable", plugin(dotenv_macros))]

extern crate quickcheck;
#[macro_use] extern crate assert_matches;
#[macro_use] extern crate diesel;
#[cfg(feature = "unstable")]
#[macro_use] extern crate diesel_codegen;

#[cfg(feature = "unstable")]
include!("lib.in.rs");

#[cfg(not(feature = "unstable"))]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

mod boxed_queries;
mod connection;
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
