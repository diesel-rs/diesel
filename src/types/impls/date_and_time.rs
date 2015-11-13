extern crate byteorder;

use std::error::Error;
use std::io::Write;

use expression::*;
use expression::bound::Bound;
use query_source::Queriable;
use types::{self, NativeSqlType, FromSql, ToSql, IsNull};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PgTimestamp(pub i64);

primitive_impls! {
    Timestamp -> (PgTimestamp, 1114),
}

impl ToSql<types::Timestamp> for PgTimestamp {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::BigInt>::to_sql(&self.0, out)
    }
}

impl FromSql<types::Timestamp> for PgTimestamp {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        FromSql::<types::BigInt>::from_sql(bytes)
            .map(PgTimestamp)
    }
}
