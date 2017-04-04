#[cfg(feature="bigdecimal")]
mod bigdecimal {
    extern crate num_traits;
    extern crate num_bigint;
    extern crate num_integer;
    extern crate bigdecimal;

    use std::error::Error;
    use std::io::prelude::*;

    use pg::Pg;

    use self::num_traits::{Signed, Zero, ToPrimitive};
    use self::num_bigint::{Sign, BigInt, BigUint, ToBigInt};
    use self::num_integer::Integer;
    use self::bigdecimal::BigDecimal;

    use pg::data_types::PgNumeric;
    use types::{self, FromSql, ToSql, IsNull};

    impl ToSql<types::Numeric, Pg> for BigDecimal {
        fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error + Send + Sync>> {
            use std::str::FromStr;

            let absolute = self.abs();

            let mut digits = vec![];
            let ten_k = BigInt::from(10000);
            let mut integer_part = absolute.to_bigint().expect("Can always take integer part of BigDecimal");
            let decimal_part = &absolute;
            let mut decimal_part = decimal_part - absolute.with_scale(0);
            // scale is the amount of digits to print. to_string() includes a "0.",
            // that's why the -2 is there.
            let scale = if decimal_part == Zero::zero() {
                0
            } else {
                decimal_part.to_string().len() as u16 - 2
            };
            let mut weight = 0;

            // Encode the integer part
            while ten_k < integer_part {
                weight += 1;
                // digit is integer_part REM 10_000
                let (div, digit) = integer_part.div_rem(&ten_k);
                digits.push(digit.to_u16().expect("digit < 10000, but cannot fit in i16") as i16);
                integer_part = div;
            }
            digits.push(integer_part.to_string().parse::<i16>().expect("digit < 10000, but cannot fit in i16"));

            digits.reverse();

            // Encode the decimal part
            let ten_k = BigDecimal::from_str("10000").expect("Could not parse into BigDecimal");
            while decimal_part != BigDecimal::zero() {
                decimal_part *= &ten_k;
                let digit = decimal_part.to_bigint().expect("Can always take integer part of BigDecimal");
                // This can be simplified when github.com/akubera/bigdecimal-rs/issues/13 gets
                // solved; decimal_part -= &digit; should suffice by then.
                decimal_part -= BigDecimal::new(digit.clone(), 0);
                digits.push(digit.to_u16().expect("digit < 10000, but cannot fit in i16") as i16);
            }

            let numeric = match self.sign() {
                Sign::Plus => PgNumeric::Positive {
                    digits, scale, weight
                },
                Sign::Minus => PgNumeric::Negative {
                    digits, scale, weight
                },
                Sign::NoSign => PgNumeric::Positive {
                    digits: vec![0],
                    scale: 0,
                    weight: 0,
                },
            };
            ToSql::<types::Numeric, Pg>::to_sql(&numeric, out)
        }
    }

    impl FromSql<types::Numeric, Pg> for BigDecimal {
        fn from_sql(numeric: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
            let (sign, weight, _, digits) = match PgNumeric::from_sql(numeric)? {
                PgNumeric::Positive { weight, scale, digits } => (Sign::Plus, weight, scale, digits),
                PgNumeric::Negative { weight, scale, digits } => (Sign::Minus, weight, scale, digits),
                PgNumeric::NaN => return Err(Box::from("NaN is not (yet) supported in BigDecimal")),
            };
            let mut result = BigUint::default();
            let count = digits.len() as i64;
            for digit in digits {
                result = result * BigUint::from(10_000u64);
                result = result + BigUint::from(digit as u64);
            }
            // First digit got factor 10_000^(digits.len() - 1), but should get 10_000^weight
            let correction_exp = 4 * ( (weight as i64) - count + 1);
            // FIXME: `scale` allows to drop some insignificant figures, which is currently unimplemented.
            // This means that e.g. PostgreSQL 0.01 will be interpreted as 0.0100
            let result = BigDecimal::new(BigInt::from_biguint(sign, result), -correction_exp);
            Ok(result)
        }
    }
}
