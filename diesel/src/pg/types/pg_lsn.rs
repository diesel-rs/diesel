use diesel_derives::AsExpression;
use diesel_derives::FromSqlRow;

use super::sql_types;
use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

/// A type encoding a position in the PostgreSQL *Write Ahead Log* (WAL).
/// In Postgres, it is represented as an unsigned 64 bit integer.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::PgLsn)]
pub struct PgLsn(pub u64);

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::PgLsn, Pg> for PgLsn {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        if bytes.len() < 8 {
            return emit_size_error(
                "Received less than 8 bytes while decoding a pg_lsn. \
                    Was an Integer expression accidentally marked as pg_lsn?",
            );
        }

        if bytes.len() > 8 {
            return emit_size_error(
                "Received more than 8 bytes while decoding a pg_lsn. \
                    Was an expression of a different type expression accidentally marked as pg_lsn?"
            );
        }
        let val = bytes
            .read_u64::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)?;
        Ok(PgLsn(val))
    }
}

#[cold]
#[inline(never)]
fn emit_size_error<T>(var_name: &str) -> deserialize::Result<T> {
    deserialize::Result::Err(var_name.into())
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::PgLsn, Pg> for PgLsn {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_u64::<NetworkEndian>(self.0)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(test)]
#[diesel_test_helper::test]
fn lsn_roundtrip() {
    use crate::query_builder::bind_collector::ByteWrapper;

    let mut buffer = Vec::new();
    let mut bytes = Output::test(ByteWrapper(&mut buffer));
    let input_lsn = PgLsn(0x525400fbc61617ff);
    ToSql::<sql_types::PgLsn, Pg>::to_sql(&input_lsn, &mut bytes).unwrap();
    let output_lsn: PgLsn = FromSql::from_sql(PgValue::for_test(&buffer)).unwrap();
    assert_eq!(input_lsn, output_lsn);
}
