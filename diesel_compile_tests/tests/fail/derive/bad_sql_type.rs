extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
#[diesel(sql_type)]
struct Lol;

#[derive(AsExpression)]
#[diesel(sql_type(Foo))]
struct Lol2;

#[derive(AsExpression)]
#[diesel(sql_type = "foo")]
struct Lol3;

#[derive(AsExpression)]
#[diesel(sql_type = 1omg)]
struct Lol4;

fn main() {}
