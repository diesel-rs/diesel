mod date_and_time;

use std::io::prelude::*;

use deserialize::{self, FromSql};
use serialize::{self, Output, ToSql};
use super::Sqlite;
use super::connection::SqliteValue;
use sql_types;

impl FromSql<sql_types::VarChar, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        let text = not_none!(value).read_text();
        Ok(text.into())
    }
}

impl FromSql<sql_types::Binary, Sqlite> for Vec<u8> {
    fn from_sql(bytes: Option<&SqliteValue>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes).read_blob();
        Ok(bytes.into())
    }
}

impl FromSql<sql_types::SmallInt, Sqlite> for i16 {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        Ok(not_none!(value).read_integer() as i16)
    }
}

impl FromSql<sql_types::Integer, Sqlite> for i32 {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        Ok(not_none!(value).read_integer())
    }
}

impl FromSql<sql_types::Bool, Sqlite> for bool {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        Ok(not_none!(value).read_integer() != 0)
    }
}

impl FromSql<sql_types::BigInt, Sqlite> for i64 {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        Ok(not_none!(value).read_long())
    }
}

impl FromSql<sql_types::Float, Sqlite> for f32 {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        Ok(not_none!(value).read_double() as f32)
    }
}

impl FromSql<sql_types::Double, Sqlite> for f64 {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        Ok(not_none!(value).read_double())
    }
}

impl ToSql<sql_types::Bool, Sqlite> for bool {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<sql_types::Integer, Sqlite>>::to_sql(&int_value, out)
    }
}
