//! MySQL specific types

#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use byteorder::WriteBytesExt;
use mysql::{Mysql, MysqlType};
use std::error::Error as StdError;
use std::io::Write;
use types::{FromSql, HasSqlType, IsNull, Tinyint, ToSql, ToSqlOutput};

primitive_impls!(Tinyint -> (i8, mysql: (Tiny)));

impl ToSql<::types::Tinyint, Mysql> for i8 {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Mysql>,
    ) -> Result<IsNull, Box<StdError + Send + Sync>> {
        out.write_i8(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<StdError + Send + Sync>)
    }
}

impl FromSql<::types::Tinyint, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError + Send + Sync>> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

impl ToSql<::types::Bool, Mysql> for bool {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Mysql>,
    ) -> Result<IsNull, Box<StdError + Send + Sync>> {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<::types::Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<::types::Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError + Send + Sync>> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}

impl HasSqlType<::types::Date> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::Date
    }
}

impl HasSqlType<::types::Time> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::Time
    }
}

impl HasSqlType<::types::Timestamp> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::Timestamp
    }
}

impl HasSqlType<Datetime> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::DateTime
    }
}

impl HasSqlType<::types::Numeric> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::String
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
#[derive(Debug, Clone, Copy, Default, QueryId)]
pub struct Datetime;

primitive_impls!(Datetime);
