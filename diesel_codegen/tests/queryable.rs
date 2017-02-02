#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate diesel_codegen;
extern crate diesel;

mod test_helpers;

use diesel::expression::dsl::sql;
use diesel::prelude::*;
use test_helpers::connection;
use diesel::types::Integer;

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = connection();
    let data = ::diesel::select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
}

#[test]
fn tuple_struct() {

   #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
   struct MyStruct(#[column_name(foo)] i32, #[column_name(bar)] i32);

   let conn = connection();
   let data = ::diesel::select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
   assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn tuple_struct_without_column_name_annotations() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, i32);

    let conn = connection();
    let data = ::diesel::select(sql::<(Integer, Integer)>("1, 2")).get_result(&conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}
