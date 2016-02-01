#![cfg_attr(feature = "unstable", feature(custom_derive, plugin, custom_attribute, time2))]
#![cfg_attr(feature = "unstable", plugin(diesel_codegen, dotenv_macros))]

extern crate quickcheck;
#[macro_use] extern crate diesel;

#[cfg(feature = "unstable")]
include!("lib.in.rs");

#[cfg(not(feature = "unstable"))]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod associations;
#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod expressions;
mod filter;
mod filter_operators;
mod find;
mod internal_details;
#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod joins;
mod macros;
mod order;
mod perf_details;
mod schema_dsl;
mod select;
#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod transactions;
#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod types;
mod types_roundtrip;
mod debug;
