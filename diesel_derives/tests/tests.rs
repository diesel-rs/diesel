#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate diesel;

mod helpers;
mod schema;

mod as_changeset;
mod as_expression;
mod associations;
mod auto_type;
mod identifiable;
mod insertable;
mod multiconnection;
mod queryable;
mod queryable_by_name;
mod selectable;
