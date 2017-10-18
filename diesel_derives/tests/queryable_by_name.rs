use diesel::*;

use test_helpers::connection;

#[cfg(feature = "mysql")]
type IntSql = ::diesel::types::BigInt;
#[cfg(feature = "mysql")]
type IntRust = i64;

#[cfg(not(feature = "mysql"))]
type IntSql = ::diesel::types::Integer;
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
    struct MyStruct(#[column_name(foo)] IntRust, #[column_name(bar)] IntRust);

    let conn = connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

// FIXME: Test usage with renamed columns
