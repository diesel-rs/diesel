#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::*;
use diesel::sql_types::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    let _ = users.select((max(id), count(users.star())));
    // No error, multiple aggregates are fine

    let _ = users.select((id, count(users.star())));
    //~^ ERROR E0277

    sql_function!(takes_two_args, takes_two_args_t, (x: Nullable<Integer>, y: Nullable<Integer>));
    let _ = users.select(takes_two_args(max(id), min(id)));
    // No error, multiple aggregates are fine

    let _ = users.select(takes_two_args(max(id), id.nullable()));
    //~^ ERROR asdf

    let _ = users.select((count(users.star()), takes_two_args(max(id), min(id))));
    // No error, multiple aggregates are fine

    let _ = users.select((id, takes_two_args(max(id), min(id))));
    //~^ ERROR asdf
    let _ = users.select((count(users.star()), takes_two_args(max(id), id)));
    //~^ ERROR asdf
}
