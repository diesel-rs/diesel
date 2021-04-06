extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
#[sql_type(Foo)]
#[sql_type]
#[sql_type = "@%&&*"]
#[sql_type = "1omg"]
struct Lol;

fn main() {}
