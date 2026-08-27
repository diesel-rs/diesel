#![allow(clippy::cast_sign_loss)] // test code

use quickcheck::{Arbitrary, Gen};

use super::PgNumeric;

const SCALE_MASK: u16 = 0x3FFF;

impl Arbitrary for PgNumeric {
    fn arbitrary(g: &mut Gen) -> Self {
        match g
            .choose(&[0u8, 1, 2, 3, 4])
            .copied()
            .expect("the slice is not empty")
        {
            2 => return PgNumeric::NaN,
            3 => return PgNumeric::PositiveInfinity,
            4 => return PgNumeric::NegativeInfinity,
            _ => {}
        }

        let mut positive = bool::arbitrary(g);
        let mut weight = -1;
        while weight < 0 {
            // Oh postgres... Don't ever change. https://bit.ly/lol-code-comments
            weight = i16::arbitrary(g);
        }
        let scale = u16::arbitrary(g) & SCALE_MASK;
        let weight_u16 = u16::try_from(weight).expect("weight is non-negative");
        let digits = gen_vec_of_appropriate_length_valid_digits(g, weight_u16, scale);
        if digits.is_empty() {
            weight = 0;
            positive = true;
        }

        match positive {
            true => PgNumeric::Positive {
                digits,
                weight,
                scale,
            },
            false => PgNumeric::Negative {
                digits,
                weight,
                scale,
            },
        }
    }
}

fn gen_vec_of_appropriate_length_valid_digits(g: &mut Gen, weight: u16, scale: u16) -> Vec<i16> {
    let max_digits = ::core::cmp::min(weight, scale);
    let mut digits = Vec::<Digit>::arbitrary(g)
        .into_iter()
        .map(|d| d.0)
        .skip_while(|d| d == &0) // drop leading zeros
        .take(usize::from(max_digits))
        .collect::<Vec<_>>();
    while digits.last() == Some(&0) {
        digits.pop(); // drop trailing zeros
    }
    digits
}

#[derive(Debug, Clone, Copy)]
struct Digit(i16);

impl Arbitrary for Digit {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut n = -1;
        while !(0..10_000).contains(&n) {
            n = i16::arbitrary(g);
        }
        Digit(n)
    }
}
