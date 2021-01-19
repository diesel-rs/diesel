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
    let _ = users::table.filter(sum(users::id).eq(1));
}
