extern crate quickcheck;
#[macro_use] extern crate assert_matches;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate dotenv;

#[cfg(not(feature = "sqlite"))]
mod annotations;
mod associations;
mod boxed_queries;
mod connection;
#[cfg(feature = "postgres")]
mod custom_schemas;
mod debug;
mod delete;
mod deserialization;
mod errors;
mod expressions;
mod filter;
mod filter_operators;
mod find;
mod group_by;
mod insert;
mod internal_details;
mod joins;
mod macros;
mod order;
mod perf_details;
mod schema;
mod schema_dsl;
mod schema_inference;
mod select;
mod transactions;
mod types;
mod types_roundtrip;
mod update;
