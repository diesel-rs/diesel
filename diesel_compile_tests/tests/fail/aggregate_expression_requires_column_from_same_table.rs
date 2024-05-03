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
    use diesel::dsl::*;
    let source = users::table.select(sum(posts::id));
    let source = users::table.select(avg(posts::id));
    let source = users::table.select(max(posts::id));
    let source = users::table.select(min(posts::id));
}
