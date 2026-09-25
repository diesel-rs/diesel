//! Mariadb-specific `RETURNING` clause helpers.
//!
//! This module exposes [`old_value()`], the wrapper used to refer to the
//! pre-modification value of a column in a Mariadb `UPDATE ... RETURNING` clause —
//! the `RETURNING OLD_VALUE(col)` syntax introduced in Mariadb 13.0.

mod old_impl;

pub use self::old_impl::{OldValueOf, old_value};
