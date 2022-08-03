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
    #[diesel(table_name = my_structs)]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = &mut connection();
    let data = my_structs::table
        .select(MyStruct::as_select())
        .get_result(conn);
    assert!(data.is_err());
}

#[test]
fn tuple_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct MyStruct(
        #[diesel(column_name = foo)] i32,
        #[diesel(column_name = bar)] i32,
    );

    let conn = &mut connection();
    let data = my_structs::table
        .select(MyStruct::as_select())
        .get_result(conn);
    assert!(data.is_err());
}

// FIXME: Test usage with renamed columns

#[test]
fn embedded_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct A<B> {
        foo: i32,
        #[diesel(embed)]
        b: B,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct C {
        bar: i32,
    }

    let conn = &mut connection();
    let data = my_structs::table
        .select(A::<C>::as_select())
        .get_result(conn);
    assert!(data.is_err());
}

#[test]
fn embedded_option() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: Option<B>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct B {
        bar: i32,
    }

    let conn = &mut connection();
    let data = my_structs::table.select(A::as_select()).get_result(conn);
    assert!(data.is_err());
}

#[test]
fn embedded_option_with_nullable_field() {
    table! {
        my_structs (foo) {
            foo -> Integer,
            bar -> Nullable<Integer>,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: Option<B>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct B {
        bar: Option<i32>,
    }

    let conn = &mut connection();
    let data = my_structs::table.select(A::as_select()).get_result(conn);
    assert!(data.is_err());
}

#[test]
fn manually_specified_expression() {
    table! {
        my_structs (foo) {
            foo -> Integer,
            bar -> Nullable<Text>,
            some_int -> Nullable<Integer>,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct A {
        foo: i32,
        #[diesel(
            select_expression = (my_structs::bar.is_not_null(), my_structs::some_int),
            select_expression_type = (dsl::IsNotNull<my_structs::bar>, my_structs::some_int),
        )]
        bar_is_set_and_some_int: (bool, Option<i32>),
        #[diesel(
            select_expression = my_structs::bar.is_not_null(),
            select_expression_type =  dsl::IsNotNull<my_structs::bar>,
        )]
        bar_is_set: bool,
    }

    let conn = &mut connection();
    let data = my_structs::table.select(A::as_select()).get_result(conn);
    assert!(data.is_err());
}
