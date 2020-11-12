#[macro_use]
extern crate diesel;

table! {
    foo {
        id -> Integer,
    }
}

#[derive(Identifiable)]
#[diesel(table_name = foo)]
struct Foo1 {}

#[derive(Identifiable)]
#[diesel(table_name = foo)]
struct Foo2 {
    #[diesel(column_name = foo)]
    id: i32,
}

#[derive(Identifiable)]
#[diesel(primary_key(bar))]
#[diesel(table_name = foo)]
struct Foo3 {}

#[derive(Identifiable)]
#[diesel(primary_key(baz))]
#[diesel(table_name = foo)]
struct Foo4 {
    #[diesel(column_name = bar)]
    baz: i32,
}

#[derive(Identifiable)]
#[diesel(primary_key(foo, bar))]
#[diesel(table_name = foo)]
struct Foo5 {
    foo: i32,
}

#[derive(Identifiable)]
#[diesel(primary_key(foo, bar))]
#[diesel(table_name = foo)]
struct Foo6 {
    foo: i32,
    #[diesel(column_name = baz)]
    bar: i32,
}

fn main() {}
