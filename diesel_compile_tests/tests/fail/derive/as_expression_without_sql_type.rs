extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
//~^ ERROR: At least one `sql_type` is needed for deriving `AsExpression` on a structure.
struct Lol;

fn main() {}
