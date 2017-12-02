#[macro_use] extern crate diesel;

#[derive(QueryableByName)]
//~^ ERROR Your struct must either be annotated with `#[table_name = "foo"]` or have all of its fields annotated with `#[sql_type = "Integer"]`
struct Foo {
    a: i32,
}

fn main() {}
