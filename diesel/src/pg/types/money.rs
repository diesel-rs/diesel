//! Support for Money values under PostgreSQL.
use std::error::Error;
use std::io::prelude::*;

use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};

/// Money is represented in Postgres as a 64 bit signed integer. The fractional precision of the
/// value is determined by the [`lc_monetary` setting of the database](https://www.postgresql.org/docs/9.6/static/datatype-money.html).
/// This struct is a dumb wrapper type, meant only to indicate the integer's meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PgMoney(pub i64);

use pg::Pg;
use types::{self, ToSql, IsNull, FromSql};

// https://github.com/postgres/postgres/blob/502a3832cc54c7115dacb8a2dae06f0620995ac6/src/include/catalog/pg_type.h#L429-L432
primitive_impls!(Money -> (PgMoney, pg: (790, 791)));

impl FromSql<types::Money, Pg> for PgMoney {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i64::<NetworkEndian>().map(PgMoney).map_err(|e| e.into())
    }
}

impl ToSql<types::Money, Pg> for PgMoney {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_i64::<NetworkEndian>(self.0)
            .map(|_| IsNull::No)
            .map_err(|e| e.into())
    }
}

#[cfg(feature = "quickcheck")]
mod quickcheck_impls {
    extern crate quickcheck;

    use self::quickcheck::{Arbitrary, Gen};
    use super::PgMoney;

    impl Arbitrary for PgMoney {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            PgMoney(i64::arbitrary(g))
        }
    }
}

#[test]
fn cents_to_sql() {
    let mut bytes = vec![];
    let test_cents = PgMoney(72624976668147840);
    ToSql::<types::Money, Pg>::to_sql(&test_cents, &mut bytes).unwrap();
    assert_eq!(bytes,
               [0x1, 0x1 << 1, 0x1 << 2, 0x1 << 3, 0x1 << 4, 0x1 << 5, 0x1 << 6, 0x1 << 7]);
}

#[test]
fn some_cents_from_sql() {
    let input = [0x1, 0x1 << 1, 0x1 << 2, 0x1 << 3, 0x1 << 4, 0x1 << 5, 0x1 << 6, 0x1 << 7];
    let output_cents: PgMoney = FromSql::<types::Money, Pg>::from_sql(Some(&input)).unwrap();
    assert_eq!(output_cents, PgMoney(72624976668147840));
}

#[test]
fn bad_cents_from_sql() {
    let undersized = [0x1 << 1, 0x1 << 2, 0x1 << 3, 0x1 << 4, 0x1 << 5, 0x1 << 6, 0x1 << 7];
    let bad_cents: Result<PgMoney, _> = FromSql::<types::Money, Pg>::from_sql(Some(&undersized));
    assert_eq!(bad_cents.unwrap_err().description(),
               "failed to fill whole buffer");
}

#[test]
fn no_cents_from_sql() {
    let no_cents: Result<PgMoney, Box<Error + Send + Sync>> =
        FromSql::<types::Money, Pg>::from_sql(None);
    assert_eq!(no_cents.unwrap_err().description(),
               "Unexpected null for non-null column");
}
