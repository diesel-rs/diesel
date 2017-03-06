//! Support for Money values under PostgreSQL.
use std::error::Error;
use std::ops::{Add, AddAssign, Sub, SubAssign};
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

impl Add for PgMoney {
    type Output = Self;
    /// # Panics
    ///
    /// Performs a checked addition, and will `panic!` on overflow in both `debug` and `release`.
    fn add(self, rhs: PgMoney) -> Self::Output {
        self.0.checked_add(rhs.0).map(PgMoney).expect("overflow adding money amounts")
    }
}

impl AddAssign for PgMoney {
    /// # Panics
    ///
    /// Performs a checked addition, and will `panic!` on overflow in both `debug` and `release`.
    fn add_assign(&mut self, rhs: PgMoney) {
        self.0 = self.0.checked_add(rhs.0).expect("overflow adding money amounts")
    }
}

impl Sub for PgMoney {
    type Output = Self;
    /// # Panics
    ///
    /// Performs a checked subtraction, and will `panic!` on underflow in both `debug` and `release`.
    fn sub(self, rhs: PgMoney) -> Self::Output {
        self.0.checked_sub(rhs.0).map(PgMoney).expect("underflow subtracting money amounts")
    }
}

impl SubAssign for PgMoney {
    /// # Panics
    ///
    /// Performs a checked subtraction, and will `panic!` on underflow in both `debug` and `release`.
    fn sub_assign(&mut self, rhs: PgMoney) {
        self.0 = self.0.checked_sub(rhs.0).expect("underflow subtracting money amounts")
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
fn add_money() {
    let c1 = PgMoney(123);
    let c2 = PgMoney(456);
    assert_eq!(PgMoney(579), c1 + c2);
}

#[test]
fn add_assign_money() {
    let mut c1 = PgMoney(123);
    c1 += PgMoney(456);
    assert_eq!(PgMoney(579), c1);
}

#[test]
#[should_panic(expected = "overflow adding money amounts")]
fn add_money_overflow() {
    let c1 = PgMoney(::std::i64::MAX);
    let c2 = PgMoney(1);
    let _overflow = c1 + c2;
}

#[test]
#[should_panic(expected = "overflow adding money amounts")]
fn add_assign_money_overflow() {
    let mut c1 = PgMoney(::std::i64::MAX);
    c1 += PgMoney(1);
}

#[test]
fn sub_money() {
    let c1 = PgMoney(123);
    let c2 = PgMoney(456);
    assert_eq!(PgMoney(-333), c1 - c2);
}

#[test]
fn sub_assign_money() {
    let mut c1 = PgMoney(123);
    c1 -= PgMoney(456);
    assert_eq!(PgMoney(-333), c1);
}

#[test]
#[should_panic(expected = "underflow subtracting money amounts")]
fn sub_money_underflow() {
    let c1 = PgMoney(::std::i64::MIN);
    let c2 = PgMoney(1);
    let _underflow = c1 - c2;
}

#[test]
#[should_panic(expected = "underflow subtracting money amounts")]
fn sub_assign_money_underflow() {
    let mut c1 = PgMoney(::std::i64::MIN);
    c1 -= PgMoney(1);
}
