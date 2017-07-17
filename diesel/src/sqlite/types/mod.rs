mod date_and_time;

use std::io::prelude::*;
use std::error::Error;

use super::Sqlite;
use super::connection::SqliteValue;
use types::{self, FromSql, ToSql, ToSqlOutput, IsNull};

impl FromSql<types::VarChar, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        let text = not_none!(value).read_text();
        Ok(text.into())
    }
}

impl FromSql<types::Binary, Sqlite> for Vec<u8> {
    fn from_sql(bytes: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        let bytes = not_none!(bytes).read_blob();
        Ok(bytes.into())
    }
}

impl FromSql<types::SmallInt, Sqlite> for i16 {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(value).read_integer() as i16)
    }
}

impl FromSql<types::Integer, Sqlite> for i32 {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(value).read_integer())
    }
}

impl FromSql<types::Bool, Sqlite> for bool {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(value).read_integer() != 0)
    }
}

impl FromSql<types::BigInt, Sqlite> for i64 {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(value).read_long())
    }
}

impl FromSql<types::Float, Sqlite> for f32 {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(value).read_double() as f32)
    }
}

impl FromSql<types::Double, Sqlite> for f64 {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(value).read_double())
    }
}

impl ToSql<types::Bool, Sqlite> for bool {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> Result<IsNull, Box<Error+Send+Sync>> {
        let int_value = if *self {
            1
        } else {
            0
        };
        <i32 as ToSql<types::Integer, Sqlite>>::to_sql(&int_value, out)
    }
}
