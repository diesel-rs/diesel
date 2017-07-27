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
    use types::{self, FromSql, ToSql, ToSqlOutput, IsNull};

    type Digits = Vec<i16>;

    fn bigdec_add_integer_part(digits: &mut Digits, absolute: &BigDecimal) -> i16 {
        let mut weight = 0;
        let ten_k = BigInt::from(10000);

        let mut integer_part = absolute.to_bigint().expect("Can always take integer part of BigDecimal");

        while ten_k <= integer_part {
            weight += 1;
            // digit is integer_part REM 10_000
            let (div, digit) = integer_part.div_rem(&ten_k);
            digits.push(digit.to_u16().expect("digit < 10000, but cannot fit in i16") as i16);
            integer_part = div;
        }
        digits.push(integer_part.to_string().parse::<i16>().expect("digit < 10000, but cannot fit in i16"));

        digits.reverse();

        weight
    }

    fn bigdec_add_decimal_part(digits: &mut Digits, absolute: &BigDecimal) -> u16 {
        use std::str::FromStr;

        let ten_k = BigDecimal::from_str("10000").expect("Could not parse into BigDecimal");

        let decimal_part = absolute;
        let mut decimal_part = decimal_part - absolute.with_scale(0);
        // scale is the amount of digits to print. to_string() includes a "0.",
        // that's why the -2 is there.
        let scale = if decimal_part == Zero::zero() {
            0
        } else {
            decimal_part.to_string().len() as u16 - 2
        };

        while decimal_part != BigDecimal::zero() {
            decimal_part *= &ten_k;
            let digit = decimal_part.to_bigint().expect("Can always take integer part of BigDecimal");

            // This can be simplified when github.com/akubera/bigdecimal-rs/issues/13 gets
            // solved; decimal_part -= &digit; should suffice by then.
            decimal_part -= BigDecimal::new(digit.clone(), 0);
            digits.push(digit.to_u16().expect("digit < 10000, but cannot fit in i16") as i16);
        }

        scale
    }

    impl ToSql<types::Numeric, Pg> for BigDecimal {
        fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error + Send + Sync>> {
            // The encoding of the BigDecimal type for PostgreSQL is a bit complicated:
            // PostgreSQL expects the data in base-10000 (so two bytes per 10k),
            // and the decimal point should lie on a boundary (as per definition of "base-10000").

            // BigDecimal, internally, holds an int vector (base-256, one byte per byte),
            // and a base (u64, base-10) shift.

            // Therefore, we split up the encoding in three parts:
            // the sign, the (integer) part before the decimal, and the part after the decimal.

            let absolute = self.abs();
            let mut digits = vec![];

            // Encode the integer part
            let weight = bigdec_add_integer_part(&mut digits, &absolute);

            // Encode the decimal part
            let scale = bigdec_add_decimal_part(&mut digits, &absolute);

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
