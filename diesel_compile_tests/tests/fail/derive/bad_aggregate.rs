extern crate diesel;

use diesel::expression::ValidGrouping;

#[derive(ValidGrouping)]
#[diesel(aggregate = true)]
//~^ ERROR: expected `,`
struct User;

fn main() {}
