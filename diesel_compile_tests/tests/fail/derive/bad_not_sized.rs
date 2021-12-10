extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
#[diesel(not_sized = true)]
struct Lol;

fn main() {}
