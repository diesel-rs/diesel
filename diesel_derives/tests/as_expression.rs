use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::expression::AsExpression;
use diesel::serialize::{Output, ToSql};
use diesel::*;
use std::convert::TryInto;
use std::io::Write;

use helpers::connection;

table! {
    my_structs (foo) {
        foo -> Integer,
        bar -> Text,
    }
}
use diesel::sql_types::Text;
#[derive(Debug, AsExpression, FromSqlRow, Clone, Copy, PartialEq)]
#[sql_type = "Text"]
struct StringArray<const N: usize>(pub [u8; N]);

impl<DB, const N: usize> FromSql<Text, DB> for StringArray<N>
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
        let string = <String as FromSql<Text, DB>>::from_sql(bytes)?;
        let bytes_array: [u8; N] = string.into_bytes().try_into().unwrap();
        Ok(StringArray(bytes_array))
    }
}

impl<DB, const N: usize> ToSql<Text, DB> for StringArray<N>
where
    DB: Backend,
    String: ToSql<Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        let string = std::str::from_utf8(&self.0).unwrap().to_owned();

        string.to_sql(out)
    }
}

#[test]
fn struct_with_sql_type() {
    #[derive(Debug, Clone, PartialEq, Queryable, Selectable)]
    #[table_name = "my_structs"]
    struct MyStruct {
        foo: i32,
        bar: StringArray<4>,
    }

    let conn = &mut connection();
    let data = my_structs::table
        .select(MyStruct::as_select())
        .get_result(conn);
    assert!(data.is_err());
}

// #[test]
// #[cfg(all(feature = "postgres", not(feature = "sqlite"), not(feature = "mysql")))]
// fn test_generic_array_type() {
//     #[derive(Debug, Clone, PartialEq, Queryable)]
//     struct MySqlItem {
//         foo: StringArray<2>,
//         bar: i32,
//     }

//     let new_generic_array_expression = GenericArray([4.4, 4.4]);

//     let conn = &mut connection();
//     let data = select(sql::<(Array<Float8>, Integer)>("[1, 2], 2")).get_result(conn);
//     assert_eq!(
//         Ok(MySqlItem {
//             foo: StringArray([29, 1]),
//             bar: 2
//         }),
//         data
//     );
// }

// ::<dyn Expression<SqlType = Array<Float8>>>

// use diesel::sql_types::{Array, Float8};
// #[derive(Debug, AsExpression, Clone, Copy, PartialEq)]
// #[sql_type = "Array<Float8>"]
// struct GenericArray<T, const N: usize>(pub [T; N]);
