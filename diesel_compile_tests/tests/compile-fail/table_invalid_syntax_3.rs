#[macro_use] extern crate diesel;

table! {
     #[foobar]
     posts {
         id -> Integer,
     }
}
// error-pattern: Invalid `table!` syntax. Please see the `table!` macro docs for more info. `https://docs.diesel.rs/diesel/macro.table.html`

fn main() {}
