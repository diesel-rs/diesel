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
    users.filter((id, name).eq_any(users.find(1)));
}
