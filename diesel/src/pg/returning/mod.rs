//! PostgreSQL-specific `RETURNING` clause helpers.
//!
//! This module exposes [`old()`], the wrapper used to refer to the
//! pre-modification value of a column in a PostgreSQL `RETURNING` clause —
//! the `RETURNING old.col` syntax introduced in PostgreSQL 18.

// Naming: we suffix with `_impl` to avoid name conflicts with the `old` re-export below.
mod old_impl;

pub use self::old_impl::old;

pub(crate) use self::old_impl::return_type_helpers_reexported;
