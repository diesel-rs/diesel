#[macro_use]
extern crate diesel;

table! {
    foo {
        id -> Integer,
    }
}

#[derive(Identifiable)]
#[table_name = "foo"]
struct Foo1 {}

#[derive(Identifiable)]
#[table_name = "foo"]
struct Foo2 {
    #[column_name = "foo"]
    id: i32,
}

#[derive(Identifiable)]
#[primary_key(bar)]
#[table_name = "foo"]
struct Foo3 {}

#[derive(Identifiable)]
#[primary_key(baz)]
#[table_name = "foo"]
struct Foo4 {
    #[column_name = "bar"]
    baz: i32,
}

#[derive(Identifiable)]
#[primary_key(foo, bar)]
#[table_name = "foo"]
struct Foo5 {
    foo: i32,
}

#[derive(Identifiable)]
#[primary_key(foo, bar)]
#[table_name = "foo"]
struct Foo6 {
    foo: i32,
    #[column_name = "baz"]
    bar: i32,
}

fn main() {}
