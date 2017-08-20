#![deny(warnings)]

extern crate chrono;
extern crate diesel;
#[macro_use]
extern crate difference;
extern crate regex;
extern crate tempdir;

mod setup;
mod support;
mod migration_generate;
mod migration_redo;
mod migration_revert;
mod migration_run;
mod migration_list;
mod database_drop;
mod database_setup;
mod database_reset;
mod exit_codes;
mod completion_generation;
mod print_schema;
