use diesel::*;
use diesel::connection::SimpleConnection;
use schema::*;

table! {
    use diesel::types::*;
    use super::MyType;
    custom_types {
        id -> Integer,
        custom_enum -> MyType,
    }
}

pub struct MyType;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "MyType"]
pub enum MyEnum {
    Foo,
    Bar,
}

mod impls_for_insert_and_query {
    use diesel::pg::Pg;
    use diesel::types::*;
    use std::error::Error;
    use std::io::Write;

    use super::{MyEnum, MyType};

    impl HasSqlType<MyType> for Pg {
        fn metadata(lookup: &Self::MetadataLookup) -> Self::TypeMetadata {
            lookup.lookup_type("my_type")
        }
    }

    impl NotNull for MyType {}
    impl SingleValue for MyType {}

    impl ToSql<MyType, Pg> for MyEnum {
        fn to_sql<W: Write>(
            &self,
            out: &mut ToSqlOutput<W, Pg>,
        ) -> Result<IsNull, Box<Error + Send + Sync>> {
            match *self {
                MyEnum::Foo => out.write_all(b"foo")?,
                MyEnum::Bar => out.write_all(b"bar")?,
            }
            Ok(IsNull::No)
        }
    }

    impl FromSql<MyType, Pg> for MyEnum {
        fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
            match not_none!(bytes) {
                b"foo" => Ok(MyEnum::Foo),
                b"bar" => Ok(MyEnum::Bar),
                _ => Err("Unrecognized enum variant".into()),
            }
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
