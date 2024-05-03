use crate::sql_types::{self, is_nullable, SqlType};

/// Marker trait for types which can be used with `MAX` and `MIN`
#[diagnostic::on_unimplemented(
    message = "expressions of the type `{Self}` cannot be ordered by the database"
)]
pub trait SqlOrd: SqlType {}

impl SqlOrd for sql_types::SmallInt {}
impl SqlOrd for sql_types::Integer {}
impl SqlOrd for sql_types::BigInt {}
impl SqlOrd for sql_types::Float {}
impl SqlOrd for sql_types::Double {}
impl SqlOrd for sql_types::Text {}
impl SqlOrd for sql_types::Date {}
impl SqlOrd for sql_types::Interval {}
impl SqlOrd for sql_types::Time {}
impl SqlOrd for sql_types::Timestamp {}
impl<T> SqlOrd for sql_types::Nullable<T> where T: SqlOrd + SqlType<IsNull = is_nullable::NotNull> {}

#[cfg(feature = "postgres_backend")]
impl SqlOrd for sql_types::Timestamptz {}
#[cfg(feature = "postgres_backend")]
impl<T: SqlOrd> SqlOrd for sql_types::Array<T> {}

#[cfg(feature = "mysql_backend")]
impl SqlOrd for sql_types::Datetime {}
#[cfg(feature = "mysql_backend")]
impl SqlOrd for sql_types::Unsigned<sql_types::SmallInt> {}
#[cfg(feature = "mysql_backend")]
impl SqlOrd for sql_types::Unsigned<sql_types::Integer> {}
#[cfg(feature = "mysql_backend")]
impl SqlOrd for sql_types::Unsigned<sql_types::BigInt> {}
