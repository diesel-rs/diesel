extern crate diesel;

use diesel::prelude::*;

table! {
     users {
         id -> Integer,
         name -> Text,
     }
}

fn main() {
    use self::users::dsl::*;
    // Sanity check that expression methods are in scope
    users.filter(id.is_not_null());
    users.filter(id.eq_any(users.select(id)));

    users.filter((id, name).is_not_null());
    //~^ ERROR: the method `is_not_null` exists for tuple `(id, name)`, but its trait bounds were not satisfied
    users.filter((id, name).eq_any(users.find(1)));
    //~^ ERROR: the method `eq_any` exists for tuple `(id, name)`, but its trait bounds were not satisfied
}
