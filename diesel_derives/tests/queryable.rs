use diesel::dsl::sql;
use diesel::sql_types::Integer;
use diesel::*;

use helpers::connection;

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(
        #[diesel(column_name = foo)] i32,
        #[diesel(column_name = bar)] i32,
    );

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn tuple_struct_without_column_name_annotations() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, i32);

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}
