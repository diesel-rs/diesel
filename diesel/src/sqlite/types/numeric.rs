#![cfg(feature = "bigdecimal")]

extern crate bigdecimal;

use self::bigdecimal::BigDecimal;

use crate::deserialize::{self, FromSql};
use crate::sql_types::{Double, Numeric};
use crate::sqlite::connection::SqliteValue;
use crate::sqlite::Sqlite;

impl FromSql<Numeric, Sqlite> for BigDecimal {
    fn from_sql(bytes: Option<&SqliteValue>) -> deserialize::Result<Self> {
        let data = <f64 as FromSql<Double, Sqlite>>::from_sql(bytes)?;
        Ok(data.into())
    }
}
