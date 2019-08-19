#[macro_use]
extern crate diesel;

table! {
    foo {
        id -> Integer,
    }
}

#[derive(Identifiable)]
#[table_name = "foo"]
struct Bar {
    id: i32,
}

#[derive(Identifiable)]
#[table_name = "foo"]
struct Baz {
    id: i32,
}

#[derive(Associations)]
#[belongs_to(Bar, Baz)]
struct Foo {
    bar_id: i32,
}

fn main() {}
