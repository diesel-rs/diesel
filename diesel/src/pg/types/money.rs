//! Support for Money values under PostgreSQL.
use std::error::Error;
use std::io::prelude::*;

use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};

/// Money is represented in Postgres as a 64 bit signed integer.  This struct is a dumb wrapper
/// type, meant only to indicate the integer's meaning.  The fractional precision of the value is
/// determined by the [`lc_monetary` setting of the database](https://www.postgresql.org/docs/9.6/static/datatype-money.html).
/// This struct is re-exported as `Cents` as a convenient and conventional expression of a typical
/// unit of 1/100th of currency. For other names or precisions, users might consider a differently
/// named `use` of the `PgMoney` struct.
///
/// ```rust
/// use diesel::data_types::PgMoney as Pence; // 1/100th unit of Pound
/// use diesel::data_types::PgMoney as Fils;  // 1/1000th unit of Dinar
/// ```
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
