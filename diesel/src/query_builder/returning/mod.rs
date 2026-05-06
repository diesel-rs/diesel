//! Helpers for type-checking and generating SQL `RETURNING` clauses.
//!
//! This module is split in two:
//!
//! * [`returning_query_source`] — the [`ReturningQuerySource`] wrapper used
//!   as the `QS` argument to `SelectableExpression`/`AppearsOnTable` when
//!   type-checking `INSERT`/`UPDATE`/`DELETE` `RETURNING` lists, the
//!   statement-kind markers (`UpdateStmt`, `InsertStmt`, `DeleteStmt`,
//!   `InsertOnConflictDoUpdateStmt`) used as its `Stmt` parameter, and the
//!   [`InsertStmtKind`] dispatch trait that picks the right marker for an
//!   `InsertStatement`'s `Values` shape.
//!
//! * [`returning_clause`] — the actual query-fragment types
//!   ([`ReturningClause<E>`] and [`NoReturningClause`]) that get embedded
//!   inside the `INSERT`/`UPDATE`/`DELETE` ASTs to emit the `RETURNING ...`
//!   SQL. This submodule is only re-exported when the
//!   `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
//!   feature is enabled.
//!
//! [`ReturningQuerySource`]: returning_query_source::ReturningQuerySource
//! [`InsertStmtKind`]: returning_query_source::InsertStmtKind
//! [`ReturningClause<E>`]: returning_clause::ReturningClause
//! [`NoReturningClause`]: returning_clause::NoReturningClause

pub mod returning_query_source;

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
pub(crate) mod returning_clause;
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub mod returning_clause;
