use std::io::prelude::*;
use std::error::Error;

use expression::AsExpression;
use expression::bound::Bound;
use pg::Pg;
use pg::data_types::PgNumeric;
use query_source::Queryable;
use types::{self, FromSql, IsNull, ToSql};

primitive_impls!(Numeric -> (PgNumeric, pg: (1700, 1231)));

impl FromSql<types::Bool, Pg> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        match bytes {
            Some(bytes) => Ok(bytes[0] != 0),
            None => Ok(false),
        }
    }
}

impl ToSql<types::Bool, Pg> for bool {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let write_result = if *self {
            out.write_all(&[1])
        } else {
            out.write_all(&[0])
        };
        write_result.map(|_| IsNull::No)
                    .map_err(|e| Box::new(e) as Box<Error>)
    }
}

#[test]
fn bool_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::Bool, Pg>::to_sql(&true, &mut bytes).unwrap();
    ToSql::<types::Bool, Pg>::to_sql(&false, &mut bytes).unwrap();
    assert_eq!(bytes, vec![1u8, 0u8]);
}

#[test]
fn bool_from_sql_treats_null_as_false() {
    let result = <bool as FromSql<types::Bool, Pg>>::from_sql(None).unwrap();
    assert!(!result);
}
