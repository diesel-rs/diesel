#[cfg(feature = "numeric")]
mod bigdecimal {
    extern crate bigdecimal;
    extern crate num_bigint;
    extern crate num_integer;
    extern crate num_traits;

    use self::bigdecimal::BigDecimal;
    use self::num_bigint::{BigInt, BigUint, Sign};
    use self::num_integer::Integer;
    use self::num_traits::{Signed, ToPrimitive, Zero};

    use crate::deserialize::{self, FromSql};
    use crate::pg::data_types::PgNumeric;
    use crate::pg::{Pg, PgValue};
    use crate::serialize::{self, Output, ToSql};
    use crate::sql_types::Numeric;

    use std::error::Error;

    /// Iterator over the digits of a big uint in base 10k.
    /// The digits will be returned in little endian order.
    struct ToBase10000(Option<BigUint>);

    impl Iterator for ToBase10000 {
        type Item = i16;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.take().map(|v| {
                let (div, rem) = v.div_rem(&BigUint::from(10_000u16));
                if !div.is_zero() {
                    self.0 = Some(div);
                }
                rem.to_i16().expect("10000 always fits in an i16")
            })
        }
    }

    #[cfg(all(feature = "postgres_backend", feature = "numeric"))]
    impl<'a> TryFrom<&'a PgNumeric> for BigDecimal {
        type Error = Box<dyn Error + Send + Sync>;

        fn try_from(numeric: &'a PgNumeric) -> deserialize::Result<Self> {
            let (sign, weight, scale, digits) = match *numeric {
                PgNumeric::Positive {
                    weight,
                    scale,
                    ref digits,
                } => (Sign::Plus, weight, scale, digits),
                PgNumeric::Negative {
                    weight,
                    scale,
                    ref digits,
                } => (Sign::Minus, weight, scale, digits),
                PgNumeric::NaN => {
                    return Err(Box::from("NaN is not (yet) supported in BigDecimal"))
                }
            };

            let mut result = BigUint::default();
            let count = digits.len() as i64;
            for digit in digits {
                result *= BigUint::from(10_000u64);
                result += BigUint::from(*digit as u64);
            }
            // First digit got factor 10_000^(digits.len() - 1), but should get 10_000^weight
            let correction_exp = 4 * (i64::from(weight) - count + 1);
            let result = BigDecimal::new(BigInt::from_biguint(sign, result), -correction_exp)
                .with_scale(i64::from(scale));
            Ok(result)
        }
    }

    #[cfg(all(feature = "postgres_backend", feature = "numeric"))]
    impl TryFrom<PgNumeric> for BigDecimal {
        type Error = Box<dyn Error + Send + Sync>;

        fn try_from(numeric: PgNumeric) -> deserialize::Result<Self> {
            (&numeric).try_into()
        }
    }

    #[cfg(all(feature = "postgres_backend", feature = "numeric"))]
    impl<'a> From<&'a BigDecimal> for PgNumeric {
        // NOTE(clippy): No `std::ops::MulAssign` impl for `BigInt`
        // NOTE(clippy): Clippy suggests to replace the `.take_while(|i| i.is_zero())`
        // with `.take_while(Zero::is_zero)`, but that's a false positive.
        // The closure gets an `&&i16` due to autoderef `<i16 as Zero>::is_zero(&self) -> bool`
        // is called. There is no impl for `&i16` that would work with this closure.
        #[allow(clippy::assign_op_pattern, clippy::redundant_closure)]
        fn from(decimal: &'a BigDecimal) -> Self {
            let (mut integer, scale) = decimal.as_bigint_and_exponent();

            // Handling of negative scale
            let scale = if scale < 0 {
                for _ in 0..(-scale) {
                    integer = integer * 10;
                }
                0
            } else {
                scale as u16
            };

            integer = integer.abs();

            // Ensure that the decimal will always lie on a digit boundary
            for _ in 0..(4 - scale % 4) {
                integer = integer * 10;
            }
            let integer = integer.to_biguint().expect("integer is always positive");

            let mut digits = ToBase10000(Some(integer)).collect::<Vec<_>>();
            digits.reverse();
            let digits_after_decimal = scale / 4 + 1;
            let weight = digits.len() as i16 - digits_after_decimal as i16 - 1;

            let unnecessary_zeroes = digits.iter().rev().take_while(|i| i.is_zero()).count();

            let relevant_digits = digits.len() - unnecessary_zeroes;
            digits.truncate(relevant_digits);

            match decimal.sign() {
                Sign::Plus => PgNumeric::Positive {
                    digits,
                    scale,
                    weight,
                },
                Sign::Minus => PgNumeric::Negative {
                    digits,
                    scale,
                    weight,
                },
                Sign::NoSign => PgNumeric::Positive {
                    digits: vec![0],
                    scale: 0,
                    weight: 0,
                },
            }
        }
    }

    #[cfg(all(feature = "postgres_backend", feature = "numeric"))]
    impl From<BigDecimal> for PgNumeric {
        fn from(bigdecimal: BigDecimal) -> Self {
            (&bigdecimal).into()
        }
    }

    #[cfg(all(feature = "postgres_backend", feature = "numeric"))]
    impl ToSql<Numeric, Pg> for BigDecimal {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
            let numeric = PgNumeric::from(self);
            ToSql::<Numeric, Pg>::to_sql(&numeric, &mut out.reborrow())
        }
    }

    #[cfg(all(feature = "postgres_backend", feature = "numeric"))]
    impl FromSql<Numeric, Pg> for BigDecimal {
        fn from_sql(numeric: PgValue<'_>) -> deserialize::Result<Self> {
            PgNumeric::from_sql(numeric)?.try_into()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::str::FromStr;

        #[test]
        fn bigdecimal_to_pgnumeric_converts_digits_to_base_10000() {
            let decimal = BigDecimal::from_str("1").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 0,
                digits: vec![1],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("10").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 0,
                digits: vec![10],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("10000").unwrap();
            let expected = PgNumeric::Positive {
                weight: 1,
                scale: 0,
                digits: vec![1],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("10001").unwrap();
            let expected = PgNumeric::Positive {
                weight: 1,
                scale: 0,
                digits: vec![1, 1],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("100000000").unwrap();
            let expected = PgNumeric::Positive {
                weight: 2,
                scale: 0,
                digits: vec![1],
            };
            assert_eq!(expected, decimal.into());
        }

        #[test]
        fn bigdecimal_to_pg_numeric_properly_adjusts_scale() {
            let decimal = BigDecimal::from_str("1").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 0,
                digits: vec![1],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("1.0").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 1,
                digits: vec![1],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("1.1").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 1,
                digits: vec![1, 1000],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("1.10").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 2,
                digits: vec![1, 1000],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("100000000.0001").unwrap();
            let expected = PgNumeric::Positive {
                weight: 2,
                scale: 4,
                digits: vec![1, 0, 0, 1],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("0.1").unwrap();
            let expected = PgNumeric::Positive {
                weight: -1,
                scale: 1,
                digits: vec![1000],
            };
            assert_eq!(expected, decimal.into());
        }

        #[test]
        fn bigdecimal_to_pg_numeric_retains_sign() {
            let decimal = BigDecimal::from_str("123.456").unwrap();
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 3,
                digits: vec![123, 4560],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("-123.456").unwrap();
            let expected = PgNumeric::Negative {
                weight: 0,
                scale: 3,
                digits: vec![123, 4560],
            };
            assert_eq!(expected, decimal.into());
        }

        #[test]
        fn bigdecimal_with_negative_scale_to_pg_numeric_works() {
            let decimal = BigDecimal::new(50.into(), -2);
            let expected = PgNumeric::Positive {
                weight: 0,
                scale: 0,
                digits: vec![5000],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::new(1.into(), -4);
            let expected = PgNumeric::Positive {
                weight: 1,
                scale: 0,
                digits: vec![1],
            };
            assert_eq!(expected, decimal.into());
        }

        #[test]
        fn bigdecimal_with_negative_weight_to_pg_numeric_works() {
            let decimal = BigDecimal::from_str("0.1000000000000000").unwrap();
            let expected = PgNumeric::Positive {
                weight: -1,
                scale: 16,
                digits: vec![1000],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("0.00315937").unwrap();
            let expected = PgNumeric::Positive {
                weight: -1,
                scale: 8,
                digits: vec![31, 5937],
            };
            assert_eq!(expected, decimal.into());

            let decimal = BigDecimal::from_str("0.003159370000000000").unwrap();
            let expected = PgNumeric::Positive {
                weight: -1,
                scale: 18,
                digits: vec![31, 5937],
            };
            assert_eq!(expected, decimal.into());
        }

        #[test]
        fn pg_numeric_to_bigdecimal_works() {
            let expected = BigDecimal::from_str("123.456").unwrap();
            let pg_numeric = PgNumeric::Positive {
                weight: 0,
                scale: 3,
                digits: vec![123, 4560],
            };
            let res: BigDecimal = pg_numeric.try_into().unwrap();
            assert_eq!(res, expected);

            let expected = BigDecimal::from_str("-56.78").unwrap();
            let pg_numeric = PgNumeric::Negative {
                weight: 0,
                scale: 2,
                digits: vec![56, 7800],
            };
            let res: BigDecimal = pg_numeric.try_into().unwrap();
            assert_eq!(res, expected);
        }
    }
}
