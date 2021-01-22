extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

fn main() {
    // Sanity check: Valid update
    update(users::table).filter(users::id.eq(1));

    update(users::table.filter(posts::id.eq(1)));

    update(users::table).filter(posts::id.eq(1));

    update(users::table).set(users::id.eq(1))
        .filter(posts::id.eq(1));
}
