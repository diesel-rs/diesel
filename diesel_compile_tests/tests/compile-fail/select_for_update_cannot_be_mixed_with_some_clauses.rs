#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    users.for_update().distinct();
    //~^ ERROR: E0599
    users.distinct().for_update();
    //~^ ERROR: E0599

    users.for_update().group_by(id);
    //~^ ERROR: E0599
    users.group_by(id).for_update();
    //~^ ERROR: E0599

    users.into_boxed().for_update();
    //~^ ERROR: E0599
    users.for_update().into_boxed();
    //~^ ERROR: E0275
}
