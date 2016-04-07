#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::AsExpression;

table! {
    numtest (n) {
        n -> Integer,
        updated_at -> Nullable<Timestamp>,
    }
}

fn main() {
    use self::numtest::dsl::*;

    let zero = AsExpression::<types::Integer>::as_expression(0);
    let stmt = numtest.select((n, zero));
    let bad_stmt = numtest.select((n));
    let union = stmt.union(bad_stmt);
    //~^ ERROR type mismatch resolving `<diesel::query_builder::SelectStatement<diesel::types::Integer, numtest::columns::n, numtest::table> as diesel::query_builder::Query>::SqlType == (diesel::types::Integer, diesel::types::Integer)`
}
