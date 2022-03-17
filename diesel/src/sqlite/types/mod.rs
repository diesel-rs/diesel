mod date_and_time;
mod numeric;

use super::connection::SqliteValue;
use super::Sqlite;
use crate::deserialize::{self, FromSql};
use crate::query_builder::QueryId;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use crate::sql_types::SqlType;

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `String`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
#[cfg(feature = "sqlite")]
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
#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Binary, Sqlite> for *const [u8] {
    fn from_sql(bytes: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let bytes = bytes.read_blob();
        Ok(bytes as *const _)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::SmallInt, Sqlite> for i16 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer() as i16)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Integer, Sqlite> for i32 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer())
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Bool, Sqlite> for bool {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_integer() != 0)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::BigInt, Sqlite> for i64 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_long())
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Float, Sqlite> for f32 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_double() as f32)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<sql_types::Double, Sqlite> for f64 {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(value.read_double())
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Bool, Sqlite> for bool {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let int_value = if *self { &1 } else { &0 };
        <i32 as ToSql<sql_types::Integer, Sqlite>>::to_sql(int_value, out)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Text, Sqlite> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Binary, Sqlite> for [u8] {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::SmallInt, Sqlite> for i16 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Integer, Sqlite> for i32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::BigInt, Sqlite> for i64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Float, Sqlite> for f32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as f64);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<sql_types::Double, Sqlite> for f64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self);
        Ok(IsNull::No)
    }
}

/// The SQLite timestamp with time zone type
///
/// ### [`ToSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
/// - [`chrono::DateTime`] with `feature = "chrono"`
///
/// ### [`FromSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
/// - [`chrono::DateTime`] with `feature = "chrono"`
///
/// [`ToSql`]: crate::serialize::ToSql
/// [`FromSql`]: crate::deserialize::FromSql
#[cfg_attr(
    feature = "chrono",
    doc = " [`chrono::NaiveDateTime`]: chrono::naive::NaiveDateTime"
)]
#[cfg_attr(
    not(feature = "chrono"),
    doc = " [`chrono::NaiveDateTime`]: https://docs.rs/chrono/0.4.19/chrono/naive/struct.NaiveDateTime.html"
)]
#[cfg_attr(feature = "chrono", doc = " [`chrono::DateTime`]: chrono::DateTime")]
#[cfg_attr(
    not(feature = "chrono"),
    doc = " [`chrono::DateTime`]: https://docs.rs/chrono/0.4.19/chrono/struct.DateTime.html"
)]
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(sqlite_type(name = "Text"))]
#[cfg(feature = "sqlite")]
pub struct Timestamptz;
