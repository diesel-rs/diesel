use diesel::connection::SimpleConnection;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::*;
use schema::*;
use std::io::Write;

table! {
    use diesel::sql_types::*;
    use super::MyType;
    custom_types {
        id -> Integer,
        custom_enum -> MyType,
    }
}

#[derive(SqlType)]
#[postgres(type_name = "my_type")]
pub struct MyType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "MyType"]
pub enum MyEnum {
    Foo,
    Bar,
}

impl ToSql<MyType, Pg> for MyEnum {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            MyEnum::Foo => out.write_all(b"foo")?,
            MyEnum::Bar => out.write_all(b"bar")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<MyType, Pg> for MyEnum {
    fn from_sql(bytes: Option<PgValue>) -> deserialize::Result<Self> {
        match not_none!(bytes).as_bytes() {
            b"foo" => Ok(MyEnum::Foo),
            b"bar" => Ok(MyEnum::Bar),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[table_name = "custom_types"]
struct HasCustomTypes {
    id: i32,
    custom_enum: MyEnum,
}

#[test]
fn custom_types_round_trip() {
    let data = vec![
        HasCustomTypes {
            id: 1,
            custom_enum: MyEnum::Foo,
        },
        HasCustomTypes {
            id: 2,
            custom_enum: MyEnum::Bar,
        },
    ];
    let connection = connection();
    connection
        .batch_execute(
            r#"
        CREATE TYPE my_type AS ENUM ('foo', 'bar');
        CREATE TABLE custom_types (
            id SERIAL PRIMARY KEY,
            custom_enum my_type NOT NULL
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
