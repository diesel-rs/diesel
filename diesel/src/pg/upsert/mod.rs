//! Types and functions related to PG's `ON CONFLICT` clause
//!
//! See [the methods on `InsertStatement`](../../query_builder/struct.InsertStatement.html#impl-1)
//! for usage examples.

mod on_conflict_actions;
mod on_conflict_clause;
mod on_conflict_extension;
mod on_conflict_target;

pub use self::on_conflict_actions::excluded;
pub use self::on_conflict_extension::*;
pub use self::on_conflict_target::on_constraint;
