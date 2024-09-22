use std::marker::PhantomData;

use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::sql_types::Text;
use diesel::*;

use crate::helpers::connection;

table! {
    my_structs (foo) {
        foo -> Integer,
        bar -> Integer,
        r#type -> Integer,
    }
}

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct MyStruct {
        foo: i32,
        bar: i32,
        r#type: i32,
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
        #[diesel(column_name = "type")] i32,
    );

    let conn = &mut connection();
    let data = my_structs::table
        .select(MyStruct::as_select())
        .get_result(conn);
    assert!(data.is_err());
}

#[test]
fn tuple_struct_raw_type() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct MyStruct(
        #[diesel(column_name = foo)] i32,
        #[diesel(column_name = bar)] i32,
        #[diesel(column_name = r#type)] i32,
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
    struct A<B, T> {
        foo: i32,
        #[diesel(embed)]
        b: B,
        #[diesel(embed)]
        t: T,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct C {
        bar: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct T {
        r#type: i32,
    }

    let conn = &mut connection();
    let data = my_structs::table
        .select(A::<C, T>::as_select())
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
        #[diesel(embed)]
        t: Option<T>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct B {
        bar: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct T {
        r#type: i32,
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
            r#type -> Nullable<Integer>,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: Option<B>,
        #[diesel(embed)]
        t: Option<T>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct B {
        bar: Option<i32>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Selectable)]
    #[diesel(table_name = my_structs)]
    struct T {
        r#type: Option<i32>,
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
            r#type -> Nullable<Text>,
            another_int -> Nullable<Integer>,
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
            select_expression_type = dsl::IsNotNull<my_structs::bar>,
        )]
        bar_is_set: bool,
        #[diesel(
            select_expression = (my_structs::r#type.is_not_null(), my_structs::another_int),
            select_expression_type = (dsl::IsNotNull<my_structs::r#type>, my_structs::another_int),
        )]
        type_is_set_and_another_int: (bool, Option<i32>),
        #[diesel(
            select_expression = my_structs::r#type.is_not_null(),
            select_expression_type = dsl::IsNotNull<my_structs::r#type>,
        )]
        type_is_set: bool,
    }

    let conn = &mut connection();
    let data = my_structs::table.select(A::as_select()).get_result(conn);
    assert!(data.is_err());
}

#[allow(dead_code)] // that's essentially a compile test
#[test]
fn check_for_backend_with_deserialize_as() {
    table! {
        tests {
            id -> Integer,
            name -> Text,
            r#type -> Text,
        }
    }

    struct MyString(String);

    impl From<String> for MyString {
        fn from(s: String) -> Self {
            Self(s)
        }
    }

    #[derive(Queryable, Selectable)]
    #[diesel(check_for_backend(crate::helpers::TestBackend))]
    struct Test {
        id: i32,
        #[diesel(deserialize_as = String)]
        name: MyString,
        #[diesel(deserialize_as = String)]
        r#type: MyString,
    }
}

#[allow(dead_code)] // that's essentially a compile test
#[test]
fn check_with_lifetime_and_type_param() {
    use std::borrow::Cow;
    table! {
        test {
            id -> Integer,
            name -> Text,
            r#type -> Text,
        }
    }

    #[derive(Queryable, Selectable)]
    #[diesel(table_name = test)]
    #[diesel(check_for_backend(crate::helpers::TestBackend))]
    pub struct Account<'n0> {
        id: i32,
        name: Cow<'n0, str>,
        r#type: Cow<'n0, str>,
    }

    #[derive(Queryable, Selectable)]
    #[diesel(table_name = test)]
    #[diesel(check_for_backend(crate::helpers::TestBackend))]
    pub struct Foo<T>
    where
        T: Copy,
    {
        name: FooInner<T>,
    }

    #[derive(FromSqlRow)]
    pub struct FooInner<T>(String, PhantomData<T>);

    impl<T> FromSql<Text, crate::helpers::TestBackend> for FooInner<T>
    where
        T: Copy,
    {
        fn from_sql(
            bytes: <crate::helpers::TestBackend as backend::Backend>::RawValue<'_>,
        ) -> deserialize::Result<Self> {
            Ok(Self(
                <String as FromSql<Text, crate::helpers::TestBackend>>::from_sql(bytes)?,
                PhantomData,
            ))
        }
    }
}
