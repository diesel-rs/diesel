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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, QueryableByColumn)]
    #[table_name = "my_structs"]
    struct MyStruct {
        foo: IntRust,
        bar: IntRust,
    }

    let conn = connection();
    let data = my_structs::table
        .select_by::<MyStruct>()
        .get_result::<MyStruct>(&conn);
    assert!(data.is_err());
}

#[test]
fn tuple_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, QueryableByColumn)]
    #[table_name = "my_structs"]
    struct MyStruct(
        #[column_name = "foo"] IntRust,
        #[table_name = "my_structs"]
        #[column_name = "bar"]
        IntRust,
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, QueryableByColumn)]
    #[table_name = "my_structs"]
    struct A {
        foo: IntRust,
        #[diesel(embed)]
        b: B,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, QueryableByColumn)]
    struct B {
        #[table_name = "my_structs"]
        bar: IntRust,
    }

    let conn = connection();
    let data = my_structs::table.select_by::<A>().get_result::<A>(&conn);
    assert!(data.is_err());
}

#[test]
fn embedded_option() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, QueryableByColumn)]
    #[table_name = "my_structs"]
    struct A {
        foo: IntRust,
        #[diesel(embed)]
        b: Option<B>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, QueryableByColumn)]
    #[table_name = "my_structs"]
    struct B {
        bar: IntRust,
    }

    let conn = connection();
    let data = my_structs::table.select_by::<A>().get_result::<A>(&conn);
    assert!(data.is_err());
}
