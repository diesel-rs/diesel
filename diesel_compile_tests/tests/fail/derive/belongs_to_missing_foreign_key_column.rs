#[macro_use]
extern crate diesel;

struct Bar;

table! {
    foo {
        id -> Integer,
    }
}

#[derive(Associations)]
#[diesel(belongs_to(Bar))]
#[diesel(table_name = foo)]
struct Foo1 {
    bar_id: i32,
}

#[derive(Associations)]
#[diesel(belongs_to(Bar, foreign_key = bar_id))]
#[diesel(table_name = foo)]
struct Foo2 {
    bar_id: i32,
}

#[derive(Associations)]
#[diesel(belongs_to(Bar))]
#[diesel(table_name = foo)]
struct Foo3 {
    #[diesel(column_name = bar_id)]
    baz_id: i32,
}

#[derive(Associations)]
#[diesel(belongs_to(Bar, foreign_key = bar_id))]
#[diesel(table_name = foo)]
struct Foo4 {
    #[diesel(column_name = bar_id)]
    baz_id: i32,
}

fn main() {}
