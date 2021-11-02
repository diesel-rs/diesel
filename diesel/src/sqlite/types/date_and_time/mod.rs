use std::io::Write;

use crate::deserialize::{self, FromSql};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types;
use crate::sqlite::connection::SqliteValue;
use crate::sqlite::Sqlite;

#[cfg(feature = "chrono")]
mod chrono;

impl FromSql<sql_types::Date, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<sql_types::Date, Sqlite> for str {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<sql_types::Date, Sqlite> for String {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        <&str as ToSql<sql_types::Date, Sqlite>>::to_sql(&&**self, out)
    }
}

impl FromSql<sql_types::Time, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<sql_types::Time, Sqlite> for str {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<sql_types::Time, Sqlite> for String {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        <&str as ToSql<sql_types::Time, Sqlite>>::to_sql(&&**self, out)
    }
}

impl FromSql<sql_types::Timestamp, Sqlite> for String {
    fn from_sql(value: SqliteValue<'_, '_>) -> deserialize::Result<Self> {
        FromSql::<sql_types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<sql_types::Timestamp, Sqlite> for str {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        ToSql::<sql_types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<sql_types::Timestamp, Sqlite> for String {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        <&str as ToSql<sql_types::Timestamp, Sqlite>>::to_sql(&&**self, out)
    }
}
