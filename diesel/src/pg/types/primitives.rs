use std::io::prelude::*;

use crate::deserialize::{self, FromSql};
use crate::pg::Pg;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

impl FromSql<sql_types::Bool, Pg> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match bytes {
            Some(bytes) => Ok(bytes[0] != 0),
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
