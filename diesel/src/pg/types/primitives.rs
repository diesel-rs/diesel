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
fn no_bool_from_sql() {
    let result = <bool as FromSql<sql_types::Bool, Pg>>::from_nullable_sql(None);
    assert_eq!(
        result.unwrap_err().to_string(),
        "Unexpected null for non-null column"
    );
}
