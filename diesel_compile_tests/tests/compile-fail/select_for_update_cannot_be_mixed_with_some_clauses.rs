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

    // FIXME: Overflows because of https://github.com/rust-lang/rust/issues/34260
    // should be E0277
    //users.for_update().distinct();
    // FIXME: Overflows because of https://github.com/rust-lang/rust/issues/34260
    // should be E0277
    // users.distinct().for_update();

    users.for_update().group_by(id);
    //~^ ERROR: E0599
    // FIXME: Overflows because of https://github.com/rust-lang/rust/issues/34260
    // should be E0277
    // users.group_by(id).for_update();

    // FIXME: Overflows because of https://github.com/rust-lang/rust/issues/34260
    // should be E0277
    // users.into_boxed().for_update();
    users.for_update().into_boxed();
    //~^ ERROR: E0275
}
