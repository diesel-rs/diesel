#![deny(warnings)]

extern crate diesel;
extern crate regex;
extern crate tempdir;
extern crate chrono;

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
