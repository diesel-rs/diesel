#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        users -> Integer,
        //~^ ERROR: Column `users` cannot be named the same as it's table.
    }
}
// error-pattern: Column `users` cannot be named the same as its table.
// error-pattern: You may use `#[sql_name = "users"]` to reference the table's `users` column.

fn main() {}
