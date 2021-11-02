use diesel::backend::Backend;
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Text;
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
