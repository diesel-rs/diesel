#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derives;

mod queryable;
mod queryable_by_name;
mod associations;
mod insertable;
mod test_helpers;
