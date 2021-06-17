#![cfg(feature = "bigdecimal")]

use bigdecimal::{BigDecimal, FromPrimitive};

use crate::deserialize::{self, FromSql};
use crate::sql_types::{Double, Numeric};
use crate::sqlite::connection::SqliteValue;
use crate::sqlite::Sqlite;

impl FromSql<Numeric, Sqlite> for BigDecimal {
    fn from_sql(bytes: &'_ SqliteValue) -> deserialize::Result<Self> {
        let x = <f64 as FromSql<Double, Sqlite>>::from_sql(bytes)?;
        BigDecimal::from_f64(x).ok_or_else(|| format!("{} is not valid decimal number ", x).into())
    }
}
