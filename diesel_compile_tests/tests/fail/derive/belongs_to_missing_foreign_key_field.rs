#[macro_use]
extern crate diesel;

struct Bar;

#[derive(Associations)]
#[diesel(belongs_to(Bar))]
//~^ ERROR: no field with column name `bar_id`
struct Foo1 {}

#[derive(Associations)]
#[diesel(belongs_to(Bar, foreign_key = bar_id))]
//~^ ERROR: no field with column name `bar_id`
struct Foo2 {}

#[derive(Associations)]
#[diesel(belongs_to(Bar))]
//~^ ERROR: no field with column name `bar_id`
struct Baz1 {
    #[diesel(column_name = baz_id)]
    bar_id: i32,
}

#[derive(Associations)]
#[diesel(belongs_to(Bar, foreign_key = bar_id))]
//~^ ERROR: no field with column name `bar_id`
struct Baz2 {
    #[diesel(column_name = baz_id)]
    bar_id: i32,
}

fn main() {}
