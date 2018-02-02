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
#[belongs_to]
#[belongs_to = "Bar"]
#[belongs_to()]
#[belongs_to(foreign_key = "bar_id")]
#[belongs_to(Bar, foreign_key)]
#[belongs_to(Bar, foreign_key(bar_id))]
#[belongs_to(Baz, foreign_key = "bar_id", random_option)]
struct Foo {
    bar_id: i32,
}

fn main() {}
