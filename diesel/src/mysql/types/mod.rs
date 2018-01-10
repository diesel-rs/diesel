//! MySQL specific types

#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use byteorder::WriteBytesExt;
use mysql::Mysql;
use std::io::Write;
use types::{self, FromSql, IsNull, ToSql, ToSqlOutput};
use {deserialize, serialize};

impl ToSql<types::Tinyint, Mysql> for i8 {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Mysql>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

impl FromSql<types::Tinyint, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

impl ToSql<types::Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Mysql>) -> serialize::Result {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<types::Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<types::Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}

/// Represents the MySQL datetime type.
///
/// ### [`ToSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
///
/// ### [`FromSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
///
/// [`ToSql`]: ../../types/trait.ToSql.html
/// [`FromSql`]: ../../types/trait.FromSql.html
/// [`chrono::NaiveDateTime`]: ../../../chrono/naive/struct.NaiveDateTime.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[mysql_type = "DateTime"]
pub struct Datetime;
