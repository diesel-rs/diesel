use diesel::dsl::sql;
use diesel::sql_types::Integer;
use diesel::*;

use crate::helpers::connection;

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
fn raw_ident_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct {
        r#foo: i32,
        #[diesel(column_name = bar)]
        r#struct: i32,
    }

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(
        Ok(MyStruct {
            foo: 1,
            r#struct: 2
        }),
        data
    );
}

#[test]
fn tuple_struct_without_column_name_annotations() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable)]
    struct MyStruct(i32, i32);

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn multiple_tables() {
    #[derive(Debug, Clone, PartialEq, Eq, Queryable)]
    #[diesel(table_name = users)]
    #[diesel(table_name = users_)]
    struct NewUser {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let data = select(sql::<(
        diesel::sql_types::Text,
        diesel::sql_types::Text,
        diesel::sql_types::Text,
    )>("'red', 'red', 'red'"))
    .get_result(conn);
    assert_eq!(
        Ok(NewUser {
            name: "red".to_string(),
            hair_color: "red".to_string(),
            r#type: "red".to_string(),
        }),
        data
    );
}

#[test]
fn name_conflict() {
    type Field = i32;
    type Record = i32;

    #[derive(Debug, Clone, PartialEq, Eq, Queryable)]
    struct MyStruct(Field, Record);

    let conn = &mut connection();
    let data = select(sql::<(Integer, Integer)>("1, 2")).get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[allow(dead_code)] // that's essentially a compile test
#[test]
fn check_can_also_use_an_associated_function_with_try_into() {
    table! {
        tests {
            id -> Integer,
            name -> Text,
            r#type -> Text,
        }
    }

    struct MyString(String);

    impl<ST, DB> Queryable<ST, DB> for MyString
    where
        DB: diesel::backend::Backend,
        String: diesel::deserialize::FromStaticSqlRow<ST, DB>,
    {
        type Row = String;

        fn build(row: Self::Row) -> deserialize::Result<Self> {
            Ok(Self(row))
        }
    }

    impl MyString {
        pub fn try_into(self) -> diesel::deserialize::Result<String> {
            Ok(self.0)
        }
    }

    #[derive(Queryable, Selectable)]
    #[diesel(check_for_backend(crate::helpers::TestBackend))]
    struct Test {
        id: i32,
        #[diesel(deserialize_as = MyString)]
        name: String,
        #[diesel(deserialize_as = MyString)]
        r#type: String,
    }
}
