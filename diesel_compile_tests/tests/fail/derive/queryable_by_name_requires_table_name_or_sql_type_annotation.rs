#[macro_use]
extern crate diesel;

#[derive(QueryableByName)]
struct Foo {
    //~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `foos`
    foo: i32,
    bar: String,
}

#[derive(QueryableByName)]
//~^ ERROR: all fields of tuple structs must be annotated with `#[diesel(column_name)]`
struct Bar(i32, String);

fn main() {}
