#![no_main]

use libfuzzer_sys::fuzz_target;

use core::num::NonZeroU32;
use diesel::deserialize::FromSql;
use diesel::pg::data_types::PgNumeric;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::Numeric;

// Fuzz diesel's PostgreSQL NUMERIC wire protocol parser.
//
// PgNumeric::from_sql parses the PG binary format:
//   [digit_count: u16] [weight: i16] [sign: u16] [scale: u16] [digits: i16...]
//
// Targets: diesel/src/pg/types/floats/mod.rs (FromSql<Numeric, Pg> for PgNumeric)

fuzz_target!(|data: &[u8]| {
    let oid = NonZeroU32::new(1700).unwrap(); // NUMERICOID
    let value = PgValue::new(data, &oid);
    let _ = <PgNumeric as FromSql<Numeric, Pg>>::from_sql(value);
});
