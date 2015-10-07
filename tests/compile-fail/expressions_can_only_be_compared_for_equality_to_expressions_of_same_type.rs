#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let pred = id.eq("string");
    //~^ ERROR E0277
    let pred = id.eq(name);
    //~^ ERROR type mismatch
}
