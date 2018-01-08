use std::io::Write;

use sqlite::Sqlite;
use sqlite::connection::SqliteValue;
use types::{self, FromSql, ToSql, ToSqlOutput};
use {deserialize, serialize};

#[cfg(feature = "chrono")]
mod chrono;

impl FromSql<types::Date, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        FromSql::<types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<types::Date, Sqlite> for str {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> serialize::Result {
        ToSql::<types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<types::Date, Sqlite> for String {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> serialize::Result {
        <&str as ToSql<types::Date, Sqlite>>::to_sql(&&**self, out)
    }
}

impl FromSql<types::Time, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        FromSql::<types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<types::Time, Sqlite> for str {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> serialize::Result {
        ToSql::<types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<types::Time, Sqlite> for String {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> serialize::Result {
        <&str as ToSql<types::Time, Sqlite>>::to_sql(&&**self, out)
    }
}

impl FromSql<types::Timestamp, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
        FromSql::<types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<types::Timestamp, Sqlite> for str {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> serialize::Result {
        ToSql::<types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<types::Timestamp, Sqlite> for String {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Sqlite>) -> serialize::Result {
        <&str as ToSql<types::Timestamp, Sqlite>>::to_sql(&&**self, out)
    }
}
