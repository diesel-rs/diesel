//! Types and functions related to PG's and Sqlite's `ON CONFLICT` clause
//!
//! Upsert is currently supported by diesel for the following database systems:
//!
//! * PostgreSQL version 9.5 or newer
//! * Sqlite3 version 3.24.0 or newer
//! * MySQL version 5.7 or newer
//!
//! See [the methods on `InsertStatement`](crate::query_builder::InsertStatement#impl-2)
//! for usage examples.
//!
//! Constructing an upsert statement from an existing select statement
//! requires a where clause on sqlite due to a ambiguity in their
//! parser. See [the corresponding documentation](https://www.sqlite.org/lang_UPSERT.html)
//! for details.
use crate::query_builder::upsert::on_conflict_actions::Excluded;

mod on_conflict_extension;

pub use self::on_conflict_extension::DecoratableTarget;
pub use self::on_conflict_extension::*;
#[cfg(feature = "postgres_backend")]
pub use crate::pg::query_builder::on_constraint::*;

/// Represents `excluded.column` in an `ON CONFLICT DO UPDATE` clause.
pub fn excluded<T>(excluded: T) -> Excluded<T> {
    Excluded::new(excluded)
}
