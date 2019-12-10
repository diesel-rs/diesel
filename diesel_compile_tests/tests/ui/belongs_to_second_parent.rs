#[macro_use]
extern crate diesel;

table! {
    bar (id) {
        id -> Integer,
    }
}

table! {
    foo (bar_id) {
        bar_id -> Integer,
    }
}

#[derive(Identifiable)]
#[table_name = "bar"]
struct Bar {
    id: i32,
}

#[derive(Identifiable)]
#[table_name = "bar"]
struct Baz {
    id: i32,
}

#[derive(Associations)]
#[belongs_to(Bar, Baz)]
#[table_name = "foo"]
struct Foo {
    bar_id: i32,
}

fn main() {}
