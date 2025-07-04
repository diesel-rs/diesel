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

    let pred = id.eq("string");
    //~^ ERROR: the trait bound `str: diesel::Expression` is not satisfied
    let pred = id.eq(name);
    //~^ ERROR: type mismatch resolving `<name as Expression>::SqlType == Integer`
}
