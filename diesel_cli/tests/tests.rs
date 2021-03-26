#![deny(warnings)]

extern crate chrono;
extern crate diesel;
#[macro_use]
extern crate difference;
extern crate regex;
extern crate tempfile;

mod completion_generation;
mod database_drop;
mod database_reset;
mod database_setup;
mod database_url_errors;
mod exit_codes;
mod migration_generate;
mod migration_list;
mod migration_redo;
mod migration_revert;
mod migration_run;
mod print_schema;
mod setup;
mod support;
