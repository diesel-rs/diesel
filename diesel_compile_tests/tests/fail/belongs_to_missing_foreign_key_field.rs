#[macro_use]
extern crate diesel;

struct Bar;

#[derive(Associations)]
#[belongs_to(Bar)]
#[belongs_to(Bar, foreign_key = "bar_id")]
struct Foo {}

#[derive(Associations)]
#[belongs_to(Bar)]
#[belongs_to(Bar, foreign_key = "bar_id")]
struct Baz {
    #[column_name = "baz_id"]
    bar_id: i32,
}

fn main() {}
