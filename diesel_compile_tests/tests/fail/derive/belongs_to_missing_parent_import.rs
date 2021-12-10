#[macro_use]
extern crate diesel;

table! {
    foos {
        id -> Integer,
        bar_id -> Integer,
    }
}

#[derive(Associations)]
#[diesel(belongs_to(Bar))]
struct Foo {
    bar_id: i32,
}

fn main() {}
