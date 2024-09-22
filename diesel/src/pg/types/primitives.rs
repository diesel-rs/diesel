use std::io::prelude::*;

use crate::deserialize::{self, FromSql, Queryable};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Bool, Pg> for bool {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(bytes.as_bytes()[0] != 0)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Bool, Pg> for bool {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(&[*self as u8])
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::CChar, Pg> for u8 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(bytes.as_bytes()[0])
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::CChar, Pg> for u8 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(&[*self])
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn cchar_to_sql() {
    use crate::query_builder::bind_collector::ByteWrapper;

    let mut buffer = Vec::new();
    let mut bytes = Output::test(ByteWrapper(&mut buffer));
    ToSql::<sql_types::CChar, Pg>::to_sql(&b'A', &mut bytes).unwrap();
    ToSql::<sql_types::CChar, Pg>::to_sql(&b'\xc4', &mut bytes).unwrap();
    assert_eq!(buffer, vec![65u8, 196u8]);
}

#[test]
fn cchar_from_sql() {
    let result = <u8 as FromSql<sql_types::CChar, Pg>>::from_nullable_sql(None);
    assert_eq!(
        result.unwrap_err().to_string(),
        "Unexpected null for non-null column"
    );
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `String`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Text, Pg> for *const str {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        use std::str;
        let string = str::from_utf8(value.as_bytes())?;
        Ok(string as *const _)
    }
}

#[cfg(feature = "postgres_backend")]
impl Queryable<sql_types::VarChar, Pg> for *const str {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Citext, Pg> for String {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Citext, Pg> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Citext, Pg> for String {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let string = String::from_utf8(value.as_bytes().to_vec())?;
        Ok(string)
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `Vec<u8>`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Binary, Pg> for *const [u8] {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(value.as_bytes() as *const _)
    }
}

#[cfg(feature = "postgres_backend")]
impl Queryable<sql_types::Binary, Pg> for *const [u8] {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[test]
fn bool_to_sql() {
    use crate::query_builder::bind_collector::ByteWrapper;

    let mut buffer = Vec::new();
    let mut bytes = Output::test(ByteWrapper(&mut buffer));
    ToSql::<sql_types::Bool, Pg>::to_sql(&true, &mut bytes).unwrap();
    ToSql::<sql_types::Bool, Pg>::to_sql(&false, &mut bytes).unwrap();
    assert_eq!(buffer, vec![1u8, 0u8]);
}

#[test]
fn no_bool_from_sql() {
    let result = <bool as FromSql<sql_types::Bool, Pg>>::from_nullable_sql(None);
    assert_eq!(
        result.unwrap_err().to_string(),
        "Unexpected null for non-null column"
    );
}
