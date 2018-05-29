use diesel::connection::SimpleConnection;
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql, WriteTuple};
use diesel::sql_types::{Integer, Record, Text};
use diesel::*;
use schema::*;
use std::io::Write;

table! {
    use diesel::sql_types::*;
    use super::MyEnumType;
    has_custom_enum {
        id -> Integer,
        custom_enum -> MyEnumType,
    }
}

#[derive(SqlType)]
#[postgres(type_name = "my_enum_type")]
pub struct MyEnumType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "MyEnumType"]
pub enum MyEnum {
    Foo,
    Bar,
}

impl ToSql<MyEnumType, Pg> for MyEnum {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            MyEnum::Foo => out.write_all(b"foo")?,
            MyEnum::Bar => out.write_all(b"bar")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<MyEnumType, Pg> for MyEnum {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"foo" => Ok(MyEnum::Foo),
            b"bar" => Ok(MyEnum::Bar),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[table_name = "has_custom_enum"]
struct HasCustomEnum {
    id: i32,
    custom_enum: MyEnum,
}

#[test]
fn custom_enum_round_trip() {
    let data = vec![
        HasCustomEnum {
            id: 1,
            custom_enum: MyEnum::Foo,
        },
        HasCustomEnum {
            id: 2,
            custom_enum: MyEnum::Bar,
        },
    ];
    let connection = connection();
    connection
        .batch_execute(
            r#"
        CREATE TYPE my_enum_type AS ENUM ('foo', 'bar');
        CREATE TABLE has_custom_enum (
            id SERIAL PRIMARY KEY,
            custom_enum my_enum_type NOT NULL
        );
    "#,
        )
        .unwrap();

    let inserted = insert_into(has_custom_enum::table)
        .values(&data)
        .get_results(&connection)
        .unwrap();
    assert_eq!(data, inserted);
}

table! {
    use diesel::sql_types::*;
    use super::MyStructType;
    has_custom_struct {
        id -> Integer,
        custom_struct -> MyStructType,
    }
}

#[derive(SqlType)]
#[postgres(type_name = "my_struct_type")]
pub struct MyStructType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "MyStructType"]
pub struct MyStruct(i32, String);

impl ToSql<MyStructType, Pg> for MyStruct {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        WriteTuple::<(Integer, Text)>::write_tuple(&(self.0, self.1.as_str()), out)
    }
}

impl FromSql<MyStructType, Pg> for MyStruct {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (num, string) = FromSql::<Record<(Integer, Text)>, Pg>::from_sql(bytes)?;
        Ok(MyStruct(num, string))
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[table_name = "has_custom_struct"]
struct HasCustomStruct {
    id: i32,
    custom_struct: MyStruct,
}

#[test]
fn custom_struct_round_trip() {
    let data = vec![
        HasCustomStruct {
            id: 1,
            custom_struct: MyStruct(1, "foo".into()),
        },
        HasCustomStruct {
            id: 2,
            custom_struct: MyStruct(2, "bar".into()),
        },
    ];
    let connection = connection();
    connection
        .batch_execute(
            r#"
        CREATE TYPE my_struct_type AS (i int4, t text);
        CREATE TABLE has_custom_struct (
            id SERIAL PRIMARY KEY,
            custom_struct my_struct_type NOT NULL
        );
    "#,
        )
        .unwrap();

    let inserted = insert_into(has_custom_struct::table)
        .values(&data)
        .get_results(&connection)
        .unwrap();
    assert_eq!(data, inserted);
}
