extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use diesel::dsl::sum;

    let _ = users::table.filter(users::name);
    //~^ ERROR: `diesel::sql_types::Text` is neither `diesel::sql_types::Bool` nor `diesel::sql_types::Nullable<Bool>`
    let _ = users::table.filter(sum(users::id).eq(1));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL
}
