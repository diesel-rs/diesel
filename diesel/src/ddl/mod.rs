//! Types to build DDL statements.
//!
//! Scoped to statements that change schema structure rather than data, each reached from
//! the schema object it acts on. Only `DROP TABLE` so far.

mod drop_table;

pub use self::drop_table::{DropTableStatement, SupportsDropTableCascade, TableDdl};

#[doc(hidden)]
pub use self::drop_table::{Cascade, IfExists, NoDropTableBehavior, NoIfExists};
