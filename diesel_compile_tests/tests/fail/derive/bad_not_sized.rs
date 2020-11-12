extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
#[diesel(sql_type = bool)]
#[diesel(not_sized = true)]
struct Lol;

fn main() {}
