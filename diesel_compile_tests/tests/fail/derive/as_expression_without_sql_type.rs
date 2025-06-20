extern crate diesel;
use diesel::expression::AsExpression;

#[derive(AsExpression)]
//~^ ERROR: at least one `sql_type` is needed for deriving `AsExpression` on a structure.
struct Lol;

fn main() {}
