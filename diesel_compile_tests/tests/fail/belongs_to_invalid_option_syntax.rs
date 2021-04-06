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
#[belongs_to]
#[belongs_to = "Bar"]
#[belongs_to()]
#[belongs_to(foreign_key = "bar_id")]
#[belongs_to(Bar, foreign_key)]
#[belongs_to(Bar, foreign_key(bar_id))]
#[belongs_to(Baz, foreign_key = "bar_id", random_option)]
#[table_name = "foo"]
struct Foo {
    bar_id: i32,
}

fn main() {}
