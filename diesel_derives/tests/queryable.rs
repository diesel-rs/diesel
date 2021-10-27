use diesel::dsl::sql;
use diesel::sql_types::{Integer, Text};
use diesel::*;

use helpers::connection;

#[test]
fn named_struct_definition() {
    #[derive(Debug, PartialEq, Eq, Queryable)]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
}

#[test]
fn tuple_struct() {
    #[derive(Debug, PartialEq, Eq, Queryable)]
    struct MyStruct(#[column_name = "foo"] i32, #[column_name = "bar"] i32);

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn tuple_struct_without_column_name_annotations() {
    #[derive(Debug, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, i32);

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn named_struct_definition_with_skip() {
    #[derive(Debug, PartialEq, Eq, Queryable)]
    struct MyStruct {
        foo: i32,
        #[diesel(skip)]
        should_be_default: Vec<i32>,
        bar: String,
    }

    let conn = &mut connection();
    let data = select(sql::<(Integer, Text)>("1, '2'")).get_result(conn);
    assert_eq!(
        Ok(MyStruct {
            foo: 1,
            should_be_default: Vec::default(),
            bar: "2".to_string(),
        }),
        data
    );
}

#[test]
fn tuple_struct_with_skip() {
    #[derive(Debug, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, #[diesel(skip)] Option<i32>, String);

    let conn = &mut connection();
    let data = select(sql::<(Integer, Text)>("1, '2'")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, None, "2".to_string())), data);
}
