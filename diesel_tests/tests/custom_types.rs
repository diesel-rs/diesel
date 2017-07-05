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

#[derive(Debug, PartialEq)]
pub enum MyEnum {
    Foo,
    Bar,
}

mod impls_for_insert_and_query {
    use diesel::expression::AsExpression;
    use diesel::expression::bound::Bound;
    use diesel::pg::Pg;
    use diesel::row::Row;
    use diesel::types::*;
    use std::error::Error;
    use std::io::Write;

    use super::{MyType, MyEnum};

    impl HasSqlType<MyType> for Pg {
        fn metadata(lookup: &Self::MetadataLookup) -> Self::TypeMetadata {
            lookup.lookup_type("my_type")
        }
    }

    impl NotNull for MyType {}

    impl<'a> AsExpression<MyType> for &'a MyEnum {
        type Expression = Bound<MyType, &'a MyEnum>;

        fn as_expression(self) -> Self::Expression {
            Bound::new(self)
        }
    }

    impl ToSql<MyType, Pg> for MyEnum {
        fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
            match *self {
                MyEnum::Foo => out.write_all(b"foo")?,
                MyEnum::Bar => out.write_all(b"bar")?,
            }
            Ok(IsNull::No)
        }
    }

    impl FromSqlRow<MyType, Pg> for MyEnum {
        fn build_from_row<T: Row<Pg>>(row: &mut T) -> Result<Self, Box<Error+Send+Sync>> {
            match row.take() {
                Some(b"foo") => Ok(MyEnum::Foo),
                Some(b"bar") => Ok(MyEnum::Bar),
                Some(_) => Err("Unrecognized enum variant".into()),
                None => Err("Unexpected null for non-null column".into()),
            }
        }
    }
}

#[derive(Insertable, Queryable, Identifiable, Debug, PartialEq)]
#[table_name="custom_types"]
struct HasCustomTypes {
    id: i32,
    custom_enum: MyEnum,
}

#[test]
fn custom_types_round_trip() {
    let data = vec![
        HasCustomTypes { id: 1, custom_enum: MyEnum::Foo },
        HasCustomTypes { id: 2, custom_enum: MyEnum::Bar },
    ];
    let connection = connection();
    connection.batch_execute(r#"
        CREATE TYPE my_type AS ENUM ('foo', 'bar');
        CREATE TABLE custom_types (
            id SERIAL PRIMARY KEY,
            custom_enum my_type NOT NULL
        );
    "#).unwrap();

    let inserted = insert(&data).into(custom_types::table)
        .get_results(&connection).unwrap();
    assert_eq!(data, inserted);
}
