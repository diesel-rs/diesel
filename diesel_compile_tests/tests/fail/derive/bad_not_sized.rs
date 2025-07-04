extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
#[diesel(not_sized = true)]
//~^ ERROR: expected `,`
struct Lol;

fn main() {}
