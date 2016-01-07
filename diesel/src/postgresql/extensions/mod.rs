//! This module contains extensions that are added to core types to aid in
//! building expressions. These traits are not exported by default. The are also
//! re-exported in `diesel::expression::dsl`
mod interval_dsl;

pub use self::interval_dsl::{MicroIntervalDsl, DayAndMonthIntervalDsl};
