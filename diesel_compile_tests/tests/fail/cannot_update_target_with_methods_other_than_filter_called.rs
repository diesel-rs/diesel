extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let command = update(users.select(id)).set(id.eq(1));
    let command = update(users.order(id)).set(id.eq(1));
}
