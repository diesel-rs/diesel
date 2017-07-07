use byteorder::WriteBytesExt;
#[cfg(feature = "chrono")]
mod date_and_time;

use mysql::{Mysql, MysqlType};
use std::error::Error as StdError;
use std::io::Write;
use types::{self, ToSql, IsNull, FromSql, HasSqlType};

/// The tinyint SQL type. This is only available on MySQL.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`i8`][i8]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`i8`][i8]
///
/// [i8]: https://doc.rust-lang.org/nightly/std/primitive.i8.html
#[derive(Debug, Clone, Copy, Default)] pub struct Tinyint;

impl ToSql<Tinyint, Mysql> for i8 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
        out.write_i8(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<StdError+Send+Sync>)
    }
}

impl FromSql<types::Tinyint, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

primitive_impls!(Tinyint -> (i8, mysql: (Tiny)));

impl ToSql<::types::Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
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
    fn metadata() -> MysqlType {
        MysqlType::Date
    }
}

impl HasSqlType<::types::Time> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Time
    }
}

impl HasSqlType<::types::Timestamp> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Timestamp
    }
}
