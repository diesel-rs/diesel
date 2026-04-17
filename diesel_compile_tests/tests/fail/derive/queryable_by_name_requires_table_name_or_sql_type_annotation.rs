#[macro_use]
extern crate diesel;

#[derive(QueryableByName)]
//~^ ERROR: all fields of tuple structs must be annotated with `#[diesel(column_name)]`
struct Bar(i32, String);

fn main() {}
