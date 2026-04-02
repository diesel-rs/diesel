extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    use self::users::dsl::*;
    use diesel::dsl::max;

    // aggregate SELECT + non-aggregate ORDER BY (the issue's example)
    let _ = users.select(max(id)).order_by(name);
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // non-aggregate SELECT + aggregate ORDER BY (also invalid SQL)
    let _ = users.select(id).order_by(max(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // aggregate SELECT + non-aggregate then_order_by
    let _ = users
        .select(max(id))
        .order_by(max(id))
        .then_order_by(name);
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL
}
