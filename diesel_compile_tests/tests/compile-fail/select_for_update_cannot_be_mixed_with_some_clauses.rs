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
    //~^ ERROR: E0271
    //~| ERROR: E0277
    //~| ERROR: E0277
    users.distinct().for_update();
    //~^ ERROR: E0271
    //~| ERROR: E0277
    users.for_update().distinct_on(id);
    //~^ ERROR: E0271
    //~| ERROR: E0277
    //~| ERROR: E0277
    //~| ERROR: E0277
    users.distinct_on(id).for_update();
    //~^ ERROR: E0271
    //~| ERROR: E0277

    users.for_update().group_by(id);
    //~^ ERROR: E0271
    //~| ERROR: E0277
    //~| ERROR: E0277
    users.group_by(id).for_update();
    //~^ ERROR: E0271
    //~| ERROR: E0277

    users.into_boxed().for_update();
    //~^ ERROR: E0271
    //~| ERROR: E0277
    users.for_update().into_boxed();
    //~^ ERROR: E0271
    //~| ERROR: E0277
    //~| ERROR: E0277
}
