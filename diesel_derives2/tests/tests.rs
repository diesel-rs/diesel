#![deny(warnings)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod helpers;
mod schema;

mod as_changeset;
