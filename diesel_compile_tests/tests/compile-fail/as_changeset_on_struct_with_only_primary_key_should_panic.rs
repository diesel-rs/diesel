#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

table!(
    foo {
        id -> Integer,
        bar -> Integer,
    }
);

#[derive(AsChangeset)]
#[table_name="foo"]
struct Foo1 {
    id: i32,
    bar: i32,
}

#[derive(AsChangeset)]
//~^ ERROR: proc-macro derive panicked
#[table_name="foo"]
struct Foo2 {
    id: i32,
}
