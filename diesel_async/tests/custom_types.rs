use crate::connection;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::{AsExpression, IntoSql};
use diesel::pg::{Pg, PgValue};
use diesel::query_builder::QueryId;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::{Array, Integer, SqlType};
use diesel::*;
use diesel_async::{RunQueryDsl, SimpleAsyncConnection};
use std::io::Write;

table! {
    use diesel::sql_types::*;
    use super::MyType;
    custom_types {
        id -> Integer,
        custom_enum -> MyType,
    }
}

#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "my_type"))]
pub struct MyType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[diesel(sql_type = MyType)]
pub enum MyEnum {
    Foo,
    Bar,
}

impl ToSql<MyType, Pg> for MyEnum {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            MyEnum::Foo => out.write_all(b"foo")?,
            MyEnum::Bar => out.write_all(b"bar")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<MyType, Pg> for MyEnum {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"foo" => Ok(MyEnum::Foo),
            b"bar" => Ok(MyEnum::Bar),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[diesel(table_name = custom_types)]
struct HasCustomTypes {
    id: i32,
    custom_enum: MyEnum,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn custom_types_round_trip() {
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
    let connection = &mut connection().await;

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
        .await
        .unwrap();

    // Try encoding arrays to test type metadata lookup
    let selected = select((
        vec![MyEnum::Foo].into_sql::<Array<MyType>>(),
        vec![0i32].into_sql::<Array<Integer>>(),
        vec![MyEnum::Bar].into_sql::<Array<MyType>>(),
    ))
    .get_result::<(Vec<MyEnum>, Vec<i32>, Vec<MyEnum>)>(connection)
    .await
    .unwrap();
    assert_eq!((vec![MyEnum::Foo], vec![0], vec![MyEnum::Bar]), selected);

    let inserted = insert_into(custom_types::table)
        .values(&data)
        .get_results(connection)
        .await
        .unwrap();
    assert_eq!(data, inserted);
}

table! {
    use diesel::sql_types::*;
    use super::MyTypeInCustomSchema;
    custom_schema.custom_types_with_custom_schema {
        id -> Integer,
        custom_enum -> MyTypeInCustomSchema,
    }
}

#[derive(SqlType, QueryId)]
#[diesel(postgres_type(name = "my_type", schema = "custom_schema"))]
pub struct MyTypeInCustomSchema;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[diesel(sql_type = MyTypeInCustomSchema)]
pub enum MyEnumInCustomSchema {
    Foo,
    Bar,
}

impl ToSql<MyTypeInCustomSchema, Pg> for MyEnumInCustomSchema {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            MyEnumInCustomSchema::Foo => out.write_all(b"foo")?,
            MyEnumInCustomSchema::Bar => out.write_all(b"bar")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<MyTypeInCustomSchema, Pg> for MyEnumInCustomSchema {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"foo" => Ok(MyEnumInCustomSchema::Foo),
            b"bar" => Ok(MyEnumInCustomSchema::Bar),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[diesel(table_name = custom_types_with_custom_schema)]
struct HasCustomTypesInCustomSchema {
    id: i32,
    custom_enum: MyEnumInCustomSchema,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn custom_types_in_custom_schema_round_trip() {
    let data = vec![
        HasCustomTypesInCustomSchema {
            id: 1,
            custom_enum: MyEnumInCustomSchema::Foo,
        },
        HasCustomTypesInCustomSchema {
            id: 2,
            custom_enum: MyEnumInCustomSchema::Bar,
        },
    ];
    let connection = &mut connection().await;
    connection
        .batch_execute(
            r#"
        CREATE SCHEMA IF NOT EXISTS custom_schema;
        CREATE TYPE custom_schema.my_type AS ENUM ('foo', 'bar');
        CREATE TABLE custom_schema.custom_types_with_custom_schema (
            id SERIAL PRIMARY KEY,
            custom_enum custom_schema.my_type NOT NULL
        );
    "#,
        )
        .await
        .unwrap();

    // Try encoding arrays to test type metadata lookup
    let selected = select((
        vec![MyEnumInCustomSchema::Foo].into_sql::<Array<MyTypeInCustomSchema>>(),
        vec![0i32].into_sql::<Array<Integer>>(),
        vec![MyEnumInCustomSchema::Bar].into_sql::<Array<MyTypeInCustomSchema>>(),
    ))
    .get_result::<(
        Vec<MyEnumInCustomSchema>,
        Vec<i32>,
        Vec<MyEnumInCustomSchema>,
    )>(connection)
    .await
    .unwrap();
    assert_eq!(
        (
            vec![MyEnumInCustomSchema::Foo],
            vec![0],
            vec![MyEnumInCustomSchema::Bar]
        ),
        selected
    );

    let inserted = insert_into(custom_types_with_custom_schema::table)
        .values(&data)
        .get_results(connection)
        .await
        .unwrap();
    assert_eq!(data, inserted);
}
