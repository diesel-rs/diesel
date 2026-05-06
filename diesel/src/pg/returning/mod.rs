//! PostgreSQL-specific `RETURNING` clause helpers.
//!
//! This module exposes [`old`] / [`Old`], the wrapper used to refer to the
//! pre-modification value of a column in a PostgreSQL `RETURNING` clause —
//! the `RETURNING old.col` syntax introduced in PostgreSQL 18.

mod old;

pub use self::old::{Old, old};
