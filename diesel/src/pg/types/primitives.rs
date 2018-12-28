use std::io::prelude::*;

use deserialize::{self, FromSql};
use pg::{Pg, PgValue};
use serialize::{self, IsNull, Output, ToSql};
use sql_types;

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `String`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
impl FromSql<sql_types::Text, Pg> for *const str {
    fn from_sql<'a>(bytes: Option<PgValue>) -> deserialize::Result<Self> {
        use std::str;
        let value = not_none!(bytes);
        let string = str::from_utf8(value.bytes())?;
        Ok(string as *const _)
    }
}

impl FromSql<sql_types::Bool, Pg> for bool {
    fn from_sql(value: Option<PgValue>) -> deserialize::Result<Self> {
        match value {
            Some(value) => Ok(value.bytes()[0] != 0),
            None => Ok(false),
        }
    }
}

impl ToSql<sql_types::Bool, Pg> for bool {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(&[*self as u8])
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn bool_to_sql() {
    let mut bytes = Output::test();
    ToSql::<sql_types::Bool, Pg>::to_sql(&true, &mut bytes).unwrap();
    ToSql::<sql_types::Bool, Pg>::to_sql(&false, &mut bytes).unwrap();
    assert_eq!(bytes, vec![1u8, 0u8]);
}

#[test]
fn bool_from_sql_treats_null_as_false() {
    let result = <bool as FromSql<sql_types::Bool, Pg>>::from_sql(None).unwrap();
    assert!(!result);
}
