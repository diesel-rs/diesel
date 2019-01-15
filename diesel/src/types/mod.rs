// All items in this module are deprecated. They are rendered in docs.
#![allow(missing_docs)]
#![cfg(feature = "with-deprecated")]
#![deprecated(since = "1.1.0", note = "Use `sql_types`, `serialize`, or `deserialize` instead")]

#[deprecated(since = "1.1.0", note = "Use `sql_types` instead")]
pub use sql_types::*;

#[deprecated(since = "1.1.0", note = "Use `deserialize` instead")]
pub use deserialize::{FromSql, FromSqlRow};

#[deprecated(since = "1.1.0", note = "Use `serialize` instead")]
pub use serialize::{IsNull, ToSql};

#[deprecated(since = "1.1.0", note = "Use `sql_types::Bool` instead")]
pub type Bool = ::sql_types::Bool;

#[deprecated(since = "1.1.0", note = "Use `sql_types::TinyInt` instead")]
pub type TinyInt = ::sql_types::TinyInt;

#[deprecated(since = "1.1.0", note = "Use `sql_types::SmallInt` instead")]
pub type SmallInt = ::sql_types::SmallInt;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Integer` instead")]
pub type Integer = ::sql_types::Integer;

#[deprecated(since = "1.1.0", note = "Use `sql_types::BigInt` instead")]
pub type BigInt = ::sql_types::BigInt;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Float` instead")]
pub type Float = ::sql_types::Float;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Double` instead")]
pub type Double = ::sql_types::Double;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Numeric` instead")]
pub type Numeric = ::sql_types::Numeric;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Text` instead")]
pub type Text = ::sql_types::Text;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Binary` instead")]
pub type Binary = ::sql_types::Binary;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Date` instead")]
pub type Date = ::sql_types::Date;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Interval` instead")]
pub type Interval = ::sql_types::Interval;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Time` instead")]
pub type Time = ::sql_types::Time;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Timestamp` instead")]
pub type Timestamp = ::sql_types::Timestamp;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Datetime` instead")]
#[cfg(feature = "mysql")]
pub type Datetime = ::sql_types::Datetime;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Oid` instead")]
#[cfg(feature = "postgres")]
pub type Oid = ::sql_types::Oid;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Timestamptz` instead")]
#[cfg(feature = "postgres")]
pub type Timestamptz = ::sql_types::Timestamptz;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Array<ST>(ST)` instead")]
#[cfg(feature = "postgres")]
pub type Array<ST> = ::sql_types::Array<ST>;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Range<ST>(ST)` instead")]
#[cfg(feature = "postgres")]
pub type Range<ST> = ::sql_types::Range<ST>;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Uuid` instead")]
#[cfg(feature = "postgres")]
pub type Uuid = ::sql_types::Uuid;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Json` instead")]
#[cfg(feature = "postgres")]
pub type Json = ::sql_types::Json;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Jsonb` instead")]
#[cfg(feature = "postgres")]
pub type Jsonb = ::sql_types::Jsonb;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Money` instead")]
#[cfg(feature = "postgres")]
pub type Money = ::sql_types::Money;

#[deprecated(since = "1.1.0", note = "Use `sql_types::MacAddr` instead")]
#[cfg(feature = "postgres")]
pub type MacAddr = ::sql_types::MacAddr;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Inet` instead")]
#[cfg(feature = "postgres")]
pub type Inet = ::sql_types::Inet;

#[deprecated(since = "1.1.0", note = "Use `sql_types::Cidr` instead")]
#[cfg(feature = "postgres")]
pub type Cidr = ::sql_types::Cidr;

#[deprecated(
    since = "1.1.0",
    note = "Use `sql_types::Nullable<ST: NotNull>(ST)` instead"
)]
pub type Nullable<ST> = ::sql_types::Nullable<ST>;

#[deprecated(since = "1.1.0", note = "Use `serialize::Output` instead")]
pub type ToSqlOutput<'a, T, DB> = ::serialize::Output<'a, T, DB>;
