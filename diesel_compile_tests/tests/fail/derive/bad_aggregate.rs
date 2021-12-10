extern crate diesel;

use diesel::expression::ValidGrouping;

#[derive(ValidGrouping)]
#[diesel(aggregate = true)]
struct User;

fn main() {}
