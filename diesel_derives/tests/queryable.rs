use diesel::dsl::sql;
use diesel::*;
use diesel::sql_types::Integer;

use helpers::connection;

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
}

#[test]
fn named_struct_definition_with_default_field() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct {
        foo: i32,
        bar: i32,
        #[diesel(default)]
        default: Option<()>,
    }

    let conn = connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(
        Ok(MyStruct {
            foo: 1,
            bar: 2,
            default: None,
        }),
        data
    );
}

#[test]
fn tuple_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(#[column_name = "foo"] i32, #[column_name = "bar"] i32);

    let conn = connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn tuple_struct_with_default() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(
        #[column_name = "foo"] i32,
        #[column_name = "bar"] i32,
        #[diesel(default)] Option<()>,
    );

    let conn = connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2, None)), data);
}

#[test]
fn tuple_struct_without_column_name_annotations() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, i32);

    let conn = connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn tuple_struct_without_column_name_annotations_with_default() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, i32, #[diesel(default)] Option<()>);

    let conn = connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2, None)), data);
}
