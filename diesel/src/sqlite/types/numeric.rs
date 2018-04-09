#![cfg(feature = "bigdecimal")]

extern crate bigdecimal;

use self::bigdecimal::BigDecimal;

use deserialize::{self, FromSql};
use sqlite::Sqlite;
use sqlite::connection::SqliteValue;
use sql_types::{Double, Numeric};

impl FromSql<Numeric, Sqlite> for BigDecimal {
    fn from_sql(bytes: Option<&SqliteValue>) -> deserialize::Result<Self> {
        let data = <f64 as FromSql<Double, Sqlite>>::from_sql(bytes)?;
        Ok(data.into())
    }
}
