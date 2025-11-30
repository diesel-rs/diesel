#[macro_use]
extern crate diesel;

table! {
    foo {
        id -> Integer,
        bar -> Integer,
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = foo)]
struct Foo1 {
    id: i32,
    bar: i32,
}

#[derive(AsChangeset)]
//~^ ERROR: deriving `AsChangeset` on a structure that only contains primary keys isn't supported.
#[diesel(table_name = foo)]
struct Foo2 {
    id: i32,
}

#[derive(AsChangeset)]
//~^ ERROR: deriving `AsChangeset` on a structure that only contains primary keys isn't supported.
#[diesel(table_name = foo, primary_key(id, bar))]
struct Foo3 {
    id: i32,
    bar: i32,
}

fn main() {}
