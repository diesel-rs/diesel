extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
#[diesel(sql_type)]
//~^ ERROR: unexpected end of input, expected `=`
struct Lol;

#[derive(AsExpression)]
#[diesel(sql_type(Foo))]
//~^ ERROR: expected `=`
struct Lol2;

#[derive(AsExpression)]
#[diesel(sql_type = "foo")]
//~^ ERROR: expected identifier
struct Lol3;

#[derive(AsExpression)]
#[diesel(sql_type = 1omg)]
//~^ ERROR: expected identifier
struct Lol4;

fn main() {}
