extern crate diesel;
use diesel::deserialize::QueryableByName;

#[derive(QueryableByName)]
struct Foo {
    foo: i32,
    bar: String,
}

#[derive(QueryableByName)]
struct Bar(i32, String);

fn main() {}
