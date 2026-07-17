#[macro_use]
extern crate diesel;

#[derive(QueryableByName)]
struct Foo {
    //~^ ERROR: cannot find module or crate `foos` in this scope
    foo: i32,
    bar: String,
}

fn main() {}
