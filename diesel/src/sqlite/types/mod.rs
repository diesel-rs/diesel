mod date_and_time;
mod numeric;

use super::connection::SqliteValue;
use super::Sqlite;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `String`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
impl FromSql<sql_types::VarChar, Sqlite> for *const str {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let text = value.read_text();
        Ok(text as *const _)
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `Vec<u8>`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
impl FromSql<sql_types::Binary, Sqlite> for *const [u8] {
    fn from_sql(bytes: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let bytes = bytes.read_blob();
        Ok(bytes as *const _)
    }
}

impl FromSql<sql_types::SmallInt, Sqlite> for i16 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer() as i16)
    }
}

impl FromSql<sql_types::Integer, Sqlite> for i32 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer())
    }
}

impl FromSql<sql_types::Bool, Sqlite> for bool {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer() != 0)
    }
}

impl FromSql<sql_types::BigInt, Sqlite> for i64 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_long())
    }
}

impl FromSql<sql_types::Float, Sqlite> for f32 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_double() as f32)
    }
}

impl FromSql<sql_types::Double, Sqlite> for f64 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_double())
    }
}

impl ToSql<sql_types::Bool, Sqlite> for bool {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        let int_value = if *self { &1 } else { &0 };
        <i32 as ToSql<sql_types::Integer, Sqlite>>::to_sql(int_value, out)
    }
}

impl ToSql<sql_types::Text, Sqlite> for str {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_borrowed_string(self);
        Ok(IsNull::No)
    }
}

impl ToSql<sql_types::Binary, Sqlite> for [u8] {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_borrowed_binary(self);
        Ok(IsNull::No)
    }
}

impl ToSql<sql_types::SmallInt, Sqlite> for i16 {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_small_int(*self);
        Ok(IsNull::No)
    }
}

impl ToSql<sql_types::Integer, Sqlite> for i32 {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_int(*self);
        Ok(IsNull::No)
    }
}

impl ToSql<sql_types::BigInt, Sqlite> for i64 {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_big_int(*self);
        Ok(IsNull::No)
    }
}

impl ToSql<sql_types::Float, Sqlite> for f32 {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_float(*self);
        Ok(IsNull::No)
    }
}

impl ToSql<sql_types::Double, Sqlite> for f64 {
    fn to_sql<'a, 'b, 'c>(&'a self, out: &mut Output<'b, 'c, Sqlite>) -> serialize::Result
    where
        'a: 'b,
    {
        out.set_double(*self);
        Ok(IsNull::No)
    }
}
