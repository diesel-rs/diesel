use sql_types::{self, NotNull};

/// Marker trait for types which can be used with `MAX` and `MIN`
pub trait SqlOrd {}

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
impl<T: SqlOrd + NotNull> SqlOrd for sql_types::Nullable<T> {}

#[cfg(feature = "postgres")]
impl SqlOrd for sql_types::Timestamptz {}
#[cfg(feature = "postgres")]
impl<T: SqlOrd> SqlOrd for sql_types::Array<T> {}

#[cfg(feature = "mysql")]
impl SqlOrd for sql_types::Datetime {}
#[cfg(feature = "mysql")]
impl SqlOrd for sql_types::Unsigned<sql_types::SmallInt> {}
#[cfg(feature = "mysql")]
impl SqlOrd for sql_types::Unsigned<sql_types::Integer> {}
#[cfg(feature = "mysql")]
impl SqlOrd for sql_types::Unsigned<sql_types::BigInt> {}
