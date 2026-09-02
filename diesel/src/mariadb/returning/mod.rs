//! Mariadb-specific `RETURNING` clause helpers.
//!
//! This module exposes [`old_value()`], the wrapper used to refer to the
//! pre-modification value of a column in a Mariadb `UPDATE ... RETURNING` clause —
//! the `RETURNING OLD_VALUE(col)` syntax introduced in Mariadb 13.0.

// Naming: we suffix with `_impl` to avoid name conflicts with the `old` re-export below.
mod old_impl;

pub use self::old_impl::old_value;
