use diesel::*;

use helpers::connection;

#[cfg(feature = "mysql")]
type IntSql = ::diesel::sql_types::BigInt;
#[cfg(feature = "mysql")]
type IntRust = i64;

#[cfg(not(feature = "mysql"))]
type IntSql = ::diesel::sql_types::Integer;
#[cfg(not(feature = "mysql"))]
type IntRust = i32;

table! {
    use super::IntSql;
    my_structs (foo) {
        foo -> IntSql,
        bar -> IntSql,
    }
}

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[table_name = "my_structs"]
    struct MyStruct {
        foo: IntRust,
        bar: IntRust,
    }

    let conn = connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(&conn);
    assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
}

#[test]
fn tuple_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[table_name = "my_structs"]
    struct MyStruct(
        #[column_name = "foo"] IntRust,
        #[column_name = "bar"] IntRust,
    );

    let conn = connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

// FIXME: Test usage with renamed columns

#[test]
fn struct_with_no_table() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    struct MyStructNamedSoYouCantInferIt {
        #[sql_type = "IntSql"]
        foo: IntRust,
        #[sql_type = "IntSql"]
        bar: IntRust,
    }

    let conn = connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(&conn);
    assert_eq!(Ok(MyStructNamedSoYouCantInferIt { foo: 1, bar: 2 }), data);
}

#[test]
fn embedded_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[table_name = "my_structs"]
    struct A {
        foo: IntRust,
        #[diesel(embed)]
        b: B,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[table_name = "my_structs"]
    struct B {
        bar: IntRust,
    }

    let conn = connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(&conn);
    assert_eq!(
        Ok(A {
            foo: 1,
            b: B { bar: 2 },
        }),
        data
    );
}

#[test]
fn embedded_option() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[table_name = "my_structs"]
    struct A {
        foo: IntRust,
        #[diesel(embed)]
        b: Option<B>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[table_name = "my_structs"]
    struct B {
        bar: IntRust,
    }

    let conn = connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(&conn);
    assert_eq!(
        Ok(A {
            foo: 1,
            b: Some(B { bar: 2 }),
        }),
        data
    );
    let data = sql_query("SELECT 1 AS foo, NULL AS bar").get_result(&conn);
    assert_eq!(Ok(A { foo: 1, b: None }), data);
}
