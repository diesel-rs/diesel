use std::io::Write;

use deserialize::{self, FromSql};
use serialize::{self, Output, ToSql};
use sqlite::Sqlite;
use sqlite::connection::SqliteValue;
use sql_types;

#[cfg(feature = "chrono")]
mod chrono;

impl FromSql<sql_types::Date, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
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
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
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
    fn from_sql(value: Option<&SqliteValue>) -> deserialize::Result<Self> {
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
