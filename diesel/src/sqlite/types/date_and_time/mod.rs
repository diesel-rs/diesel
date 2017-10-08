use std::error::Error;
use std::io::Write;

use sqlite::{Sqlite, SqliteType};
use sqlite::connection::SqliteValue;
use types::{self, Date, FromSql, HasSqlType, IsNull, Time, Timestamp, ToSql, ToSqlOutput};

#[cfg(feature = "chrono")]
mod chrono;

impl HasSqlType<types::Date> for Sqlite {
    fn metadata(_: &()) -> SqliteType {
        SqliteType::Text
    }
}

impl HasSqlType<types::Time> for Sqlite {
    fn metadata(_: &()) -> SqliteType {
        SqliteType::Text
    }
}

impl HasSqlType<types::Timestamp> for Sqlite {
    fn metadata(_: &()) -> SqliteType {
        SqliteType::Text
    }
}

queryable_impls!(Date -> String);
queryable_impls!(Time -> String);
queryable_impls!(Timestamp -> String);

expression_impls!(Date -> String);
expression_impls!(Date -> str, unsized);
expression_impls!(Time -> String);
expression_impls!(Time -> str, unsized);
expression_impls!(Timestamp -> String);
expression_impls!(Timestamp -> str, unsized);

impl FromSql<types::Date, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<types::Date, Sqlite> for str {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Sqlite>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<types::Date, Sqlite> for String {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Sqlite>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        <&str as ToSql<types::Date, Sqlite>>::to_sql(&&**self, out)
    }
}

impl FromSql<types::Time, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<types::Time, Sqlite> for str {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Sqlite>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<types::Time, Sqlite> for String {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Sqlite>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        <&str as ToSql<types::Time, Sqlite>>::to_sql(&&**self, out)
    }
}

impl FromSql<types::Timestamp, Sqlite> for String {
    fn from_sql(value: Option<&SqliteValue>) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<types::Text, Sqlite>::from_sql(value)
    }
}

impl ToSql<types::Timestamp, Sqlite> for str {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Sqlite>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        ToSql::<types::Text, Sqlite>::to_sql(self, out)
    }
}

impl ToSql<types::Timestamp, Sqlite> for String {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Sqlite>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        <&str as ToSql<types::Timestamp, Sqlite>>::to_sql(&&**self, out)
    }
}
