#[cfg(feature = "chrono")]
mod date_and_time;

use mysql::{Mysql, MysqlType};
use std::error::Error as StdError;
use std::io::Write;
use types::{ToSql, ToSqlOutput, IsNull, FromSql, HasSqlType};

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
