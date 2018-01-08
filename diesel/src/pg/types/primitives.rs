use std::io::prelude::*;
use std::error::Error;

use pg::Pg;
use types::{self, FromSql, IsNull, ToSql, ToSqlOutput};

impl FromSql<types::Bool, Pg> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        match bytes {
            Some(bytes) => Ok(bytes[0] != 0),
            None => Ok(false),
        }
    }
}

impl ToSql<types::Bool, Pg> for bool {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_all(&[*self as u8])
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

#[test]
fn bool_to_sql() {
    let mut bytes = ToSqlOutput::test();
    ToSql::<types::Bool, Pg>::to_sql(&true, &mut bytes).unwrap();
    ToSql::<types::Bool, Pg>::to_sql(&false, &mut bytes).unwrap();
    assert_eq!(bytes, vec![1u8, 0u8]);
}

#[test]
fn bool_from_sql_treats_null_as_false() {
    let result = <bool as FromSql<types::Bool, Pg>>::from_sql(None).unwrap();
    assert!(!result);
}
