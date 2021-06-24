mod date_and_time;
mod numeric;

use std::io::prelude::*;

use super::connection::SqliteValue;
use super::Sqlite;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types;

impl FromSql<sql_types::VarChar, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        let text = value.read_text();
        Ok(text)
    }
}

impl FromSql<sql_types::Binary, Sqlite> for Vec<u8> {
    fn from_sql(bytes: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        let bytes = bytes.read_blob();
        Ok(bytes)
    }
}

impl FromSql<sql_types::SmallInt, Sqlite> for i16 {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer() as i16)
    }
}

impl FromSql<sql_types::Integer, Sqlite> for i32 {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer())
    }
}

impl FromSql<sql_types::Bool, Sqlite> for bool {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer() != 0)
    }
}

impl FromSql<sql_types::BigInt, Sqlite> for i64 {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_long())
    }
}

impl FromSql<sql_types::Float, Sqlite> for f32 {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_double() as f32)
    }
}

impl FromSql<sql_types::Double, Sqlite> for f64 {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_double())
    }
}

impl ToSql<sql_types::Bool, Sqlite> for bool {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<sql_types::Integer, Sqlite>>::to_sql(&int_value, out)
    }
}
