#[macro_use]
extern crate diesel;

table! {
    foo {
        id -> Integer,
    }
}

table! {
    bars {
        id -> Integer,
        foo_id -> Integer,
    }
}

#[derive(Identifiable)]
#[diesel(table_name = foo)]
struct Foo<'a> {
    id: i32,
    _marker: ::std::marker::PhantomData<&'a ()>,
}

#[derive(Associations)]
#[diesel(belongs_to(Foo<'a>))]
struct Bar {
    foo_id: i32,
}

fn main() {}
