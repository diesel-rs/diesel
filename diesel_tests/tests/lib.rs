#![recursion_limit = "1024"]
#![deny(warnings)]

#[macro_use]
extern crate assert_matches;
extern crate chrono;
#[macro_use]
extern crate diesel;
#[cfg(feature = "sqlite")]
extern crate diesel_migrations;
extern crate dotenv;
extern crate quickcheck;

#[cfg(not(feature = "sqlite"))]
mod annotations;
mod associations;
mod boxed_queries;
mod combination;
mod connection;
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
mod insert;
mod insert_from_select;
mod internal_details;
mod joins;
mod limit_offset;
mod macros;
mod order;
mod perf_details;
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
