#[macro_use]
extern crate diesel;

#[derive(AsExpression)]
#[sql_type(Foo)]
#[sql_type]
#[sql_type = "@%&&*"]
#[sql_type = "1omg"]
struct Lol;

fn main() {}
