#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use byteorder::WriteBytesExt;
use mysql::{Mysql, MysqlType};
#[cfg(not(feature="postgres"))]
use query_builder::QueryId;
use std::error::Error as StdError;
use std::io::Write;
use types::{ToSql, ToSqlOutput, IsNull, FromSql, HasSqlType, Tinyint};

primitive_impls!(Tinyint -> (i8, mysql: (Tiny)));

impl ToSql<::types::Tinyint, Mysql> for i8 {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Mysql>) -> Result<IsNull, Box<StdError+Send+Sync>> {
        out.write_i8(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<StdError+Send+Sync>)
    }
}

impl FromSql<::types::Tinyint, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

impl ToSql<::types::Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Mysql>) -> Result<IsNull, Box<StdError+Send+Sync>> {
        let int_value = if *self {
            1
        } else {
            0
        };
        <i32 as ToSql<::types::Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<::types::Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
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

#[cfg(not(feature="postgres"))]
impl QueryId for ::types::Numeric {
    type QueryId = Self;

    fn has_static_query_id() -> bool {
        true
    }
}

/// Represents the MySQL datetime type.
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
///
/// [NaiveDateTime]: https://lifthrasiir.github.io/rust-chrono/chrono/naive/datetime/struct.NaiveDateTime.html
#[derive(Debug, Clone, Copy, Default)] pub struct Datetime;
