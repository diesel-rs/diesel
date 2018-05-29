use diesel::*;
use diesel::connection::SimpleConnection;
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql, WriteTuple};
use diesel::sql_types::{Integer, Record, Text};
use schema::*;
use std::io::Write;

table! {
    use diesel::sql_types::*;
    use super::{MyEnumType, MyStructType};
    custom_types {
        id -> Integer,
        custom_enum -> MyEnumType,
        custom_struct -> MyStructType,
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

#[derive(SqlType)]
#[postgres(type_name = "my_struct_type")]
pub struct MyStructType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "MyStructType"]
pub struct MyStruct(i32, String);

impl ToSql<MyStructType, Pg> for MyStruct {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        WriteTuple::<(Integer, Text)>::write_tuple(
            &(self.0, self.1.as_str()),
            out,
        )
    }
}

impl FromSql<MyStructType, Pg> for MyStruct {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (num, string) = FromSql::<Record<(Integer, Text)>, Pg>::from_sql(bytes)?;
        Ok(MyStruct(num, string))
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[table_name = "custom_types"]
struct HasCustomTypes {
    id: i32,
    custom_enum: MyEnum,
    custom_struct: MyStruct,
}

#[test]
fn custom_types_round_trip() {
    let data = vec![
        HasCustomTypes {
            id: 1,
            custom_enum: MyEnum::Foo,
            custom_struct: MyStruct(1, "baz".into()),
        },
        HasCustomTypes {
            id: 2,
            custom_enum: MyEnum::Bar,
            custom_struct: MyStruct(2, "quux".into()),
        },
    ];
    let connection = connection();
    connection
        .batch_execute(
            r#"
        CREATE TYPE my_enum_type AS ENUM ('foo', 'bar');
        CREATE TYPE my_struct_type AS (i int4, t text);
        CREATE TABLE custom_types (
            id SERIAL PRIMARY KEY,
            custom_enum my_enum_type NOT NULL,
            custom_struct my_struct_type NOT NULL
        );
    "#,
        )
        .unwrap();

    let inserted = insert_into(custom_types::table)
        .values(&data)
        .get_results(&connection)
        .unwrap();
    assert_eq!(data, inserted);
}
