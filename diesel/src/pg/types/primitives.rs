use std::io::prelude::*;

use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

impl FromSql<sql_types::Bool, Pg> for bool {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(bytes.as_bytes()[0] != 0)
    }
}

impl ToSql<sql_types::Bool, Pg> for bool {
    fn to_sql<'a: 'b, 'b>(&'a self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(&[*self as u8])
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `String`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
impl FromSql<sql_types::Text, Pg> for *const str {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        use std::str;
        let string = str::from_utf8(value.as_bytes())?;
        Ok(string as *const _)
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `Vec<u8>`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
impl FromSql<sql_types::Binary, Pg> for *const [u8] {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(value.as_bytes() as *const _)
    }
}

#[test]
fn bool_to_sql() {
    let mut buffer = Vec::new();
    let mut bytes = Output::test(&mut buffer);
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
