#![cfg_attr(feature = "unstable", feature(custom_derive, plugin, custom_attribute))]
#![cfg_attr(feature = "unstable", plugin(diesel_codegen, dotenv_macros))]

extern crate quickcheck;
#[macro_use] extern crate diesel;

#[cfg(feature = "unstable")]
include!("lib.in.rs");

#[cfg(not(feature = "unstable"))]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

mod associations;
mod expressions;
mod connection;
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
mod debug;
