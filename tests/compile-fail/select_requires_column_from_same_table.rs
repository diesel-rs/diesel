#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Serial,
        title -> VarChar,
    }
}

fn main() {
    let connection = Connection::establish("").unwrap();
    let select_id = users::table.select(posts::id);
    //~^ ERROR type mismatch
    // ERROR expected struct `posts::table`,
    // ERROR found struct `users::table` [E0271]
}
