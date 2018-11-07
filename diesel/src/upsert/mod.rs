//! Types and functions related to PG's and Sqlite's `ON CONFLICT` clause
//! If you use upsert, you need use PostgreSQL since version 9.0 or Sqlite3 since version 3.24.0.
//!
//! See [the methods on `InsertStatement`](../../query_builder/struct.InsertStatement.html#impl-1)
//! for usage examples.

mod on_conflict_actions;
mod on_conflict_clause;
mod on_conflict_extension;
mod on_conflict_target;

pub use self::on_conflict_actions::excluded;
pub use self::on_conflict_extension::*;
pub use self::on_conflict_target::*;
