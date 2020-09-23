use diesel::*;

use helpers::connection;

table! {
    my_structs (foo) {
        foo -> Integer,
        bar -> Integer,
    }
}

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[table_name = "my_structs"]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = connection();
    let data = my_structs::table
        .select_by::<MyStruct>()
        .get_result::<MyStruct>(&conn);
    assert!(data.is_err());
}

#[test]
fn tuple_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[table_name = "my_structs"]
    struct MyStruct(
        #[column_name = "foo"] i32,
        #[table_name = "my_structs"]
        #[column_name = "bar"]
        i32,
    );

    let conn = connection();
    let data = my_structs::table
        .select_by::<MyStruct>()
        .get_result::<MyStruct>(&conn);
    assert!(data.is_err());
}

// FIXME: Test usage with renamed columns

#[test]
fn embedded_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[table_name = "my_structs"]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: B,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    struct B {
        #[table_name = "my_structs"]
        bar: i32,
    }

    let conn = connection();
    let data = my_structs::table.select_by::<A>().get_result::<A>(&conn);
    assert!(data.is_err());
}

#[test]
fn embedded_option() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[table_name = "my_structs"]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: Option<B>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[table_name = "my_structs"]
    struct B {
        bar: i32,
    }

    let conn = connection();
    let data = my_structs::table.select_by::<A>().get_result::<A>(&conn);
    assert!(data.is_err());
}
