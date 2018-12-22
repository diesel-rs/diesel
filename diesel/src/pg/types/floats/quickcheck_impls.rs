extern crate quickcheck;

use self::quickcheck::{Arbitrary, Gen};

use super::PgNumeric;

const SCALE_MASK: u16 = 0x3FFF;

impl Arbitrary for PgNumeric {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut variant = Option::<bool>::arbitrary(g);
        let mut weight = -1;
        while weight < 0 {
            // Oh postgres... Don't ever change. https://bit.ly/lol-code-comments
            weight = i16::arbitrary(g);
        }
        let scale = u16::arbitrary(g) & SCALE_MASK;
        let digits = gen_vec_of_appropriate_length_valid_digits(g, weight as u16, scale);
        if digits.is_empty() {
            weight = 0;
            variant = Some(true);
        }

        match variant {
            Some(true) => PgNumeric::Positive {
                digits: digits,
                weight: weight,
                scale: scale,
            },
            Some(false) => PgNumeric::Negative {
                digits: digits,
                weight: weight,
                scale: scale,
            },
            None => PgNumeric::NaN,
        }
    }
}

fn gen_vec_of_appropriate_length_valid_digits<G: Gen>(
    g: &mut G,
    weight: u16,
    scale: u16,
) -> Vec<i16> {
    let max_digits = ::std::cmp::min(weight, scale);
    let mut digits = Vec::<Digit>::arbitrary(g)
        .into_iter()
        .map(|d| d.0)
        .skip_while(|d| d == &0) // drop leading zeros
        .take(max_digits as usize)
        .collect::<Vec<_>>();
    while digits.last() == Some(&0) {
        digits.pop(); // drop trailing zeros
    }
    digits
}

#[derive(Debug, Clone, Copy)]
struct Digit(i16);

impl Arbitrary for Digit {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut n = -1;
        while n < 0 || n >= 10_000 {
            n = i16::arbitrary(g);
        }
        Digit(n)
    }
}
