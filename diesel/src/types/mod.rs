// All items in this module are deprecated. They are rendered in docs.
#![allow(missing_docs)]
#![cfg(feature = "with-deprecated")]
#![deprecated(since = "1.1.0", note = "Use `sql_types`, `serialize`, or `deserialize` instead")]

#[deprecated(since = "1.1.0", note = "Use `sql_types` instead")]
pub use crate::sql_types::*;

#[deprecated(since = "1.1.0", note = "Use `deserialize` instead")]
pub use crate::deserialize::{FromSql, FromSqlRow};

#[deprecated(since = "1.1.0", note = "Use `serialize` instead")]
pub use crate::serialize::{IsNull, ToSql};

#[deprecated(since = "1.1.0", note = "Use `sql_types::Bool` instead")]
pub type Bool = crate::sql_types::Bool;

#[deprecated(since = "1.1.0", note = "Use `sql_types::TinyInt` instead")]
pub type TinyInt = crate::sql_types::TinyInt;

#[deprecated(since = "1.1.0", note = "Use `sql_types::SmallInt` instead")]
pub type SmallInt = crate::sql_types::SmallInt;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Integer` instead")]
pub type Integer = crate::sql_types::Integer;

#[deprecated(since = "1.1.0", note = "Use `sql_types::BigInt` instead")]
pub type BigInt = crate::sql_types::BigInt;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Float` instead")]
pub type Float = crate::sql_types::Float;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Double` instead")]
pub type Double = crate::sql_types::Double;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Numeric` instead")]
pub type Numeric = crate::sql_types::Numeric;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Text` instead")]
pub type Text = crate::sql_types::Text;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Binary` instead")]
pub type Binary = crate::sql_types::Binary;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Date` instead")]
pub type Date = crate::sql_types::Date;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Interval` instead")]
pub type Interval = crate::sql_types::Interval;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Time` instead")]
pub type Time = crate::sql_types::Time;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Timestamp` instead")]
pub type Timestamp = crate::sql_types::Timestamp;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Datetime` instead")]
#[cfg(feature = "mysql")]
pub type Datetime = crate::sql_types::Datetime;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Oid` instead")]
#[cfg(feature = "postgres")]
pub type Oid = crate::sql_types::Oid;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Timestamptz` instead")]
#[cfg(feature = "postgres")]
pub type Timestamptz = crate::sql_types::Timestamptz;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Array<ST>(ST)` instead")]
#[cfg(feature = "postgres")]
pub type Array<ST> = crate::sql_types::Array<ST>;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Range<ST>(ST)` instead")]
#[cfg(feature = "postgres")]
pub type Range<ST> = crate::sql_types::Range<ST>;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Uuid` instead")]
#[cfg(feature = "postgres")]
pub type Uuid = crate::sql_types::Uuid;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Json` instead")]
#[cfg(feature = "postgres")]
pub type Json = crate::sql_types::Json;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Jsonb` instead")]
#[cfg(feature = "postgres")]
pub type Jsonb = crate::sql_types::Jsonb;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Money` instead")]
#[cfg(feature = "postgres")]
pub type Money = crate::sql_types::Money;

#[deprecated(since = "1.1.0", note = "Use `sql_types::MacAddr` instead")]
#[cfg(feature = "postgres")]
pub type MacAddr = crate::sql_types::MacAddr;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Inet` instead")]
#[cfg(feature = "postgres")]
pub type Inet = crate::sql_types::Inet;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Cidr` instead")]
#[cfg(feature = "postgres")]
pub type Cidr = crate::sql_types::Cidr;

#[deprecated(
    since = "1.1.0",
    note = "Use `sql_types::Nullable<ST: NotNull>(ST)` instead"
)]
pub type Nullable<ST> = crate::sql_types::Nullable<ST>;

#[deprecated(since = "1.1.0", note = "Use `serialize::Output` instead")]
pub type ToSqlOutput<'a, T, DB> = crate::serialize::Output<'a, T, DB>;
