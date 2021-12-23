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

    let _ = users.select(name + name);
}
