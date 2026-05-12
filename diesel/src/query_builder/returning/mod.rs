//! Helpers for type-checking and generating SQL `RETURNING` clauses.
//!
//! This module is split in two:
//!
//! * [`returning_query_source`] — the [`ReturningQuerySource`] wrapper used
//!   as the `QS` argument to `SelectableExpression`/`AppearsOnTable` when
//!   type-checking `INSERT`/`UPDATE`/`DELETE` `RETURNING` lists, the
//!   statement-kind markers (`UpdateStmt`, `DeleteStmt`,
//!   `InsertStmtWithoutOnConflictDoUpdate`,
//!   `InsertStmtWithOnConflictDoUpdate`) used as its `StmtKind` parameter,
//!   and the [`InsertStmtKind`] dispatch trait that picks the right marker
//!   for an `InsertStatement`'s `Values` shape.
//!
//! * [`returning_clause`] — the actual query-fragment types
//!   ([`ReturningClause<E>`] and [`NoReturningClause`]) that get embedded in
//!   the `INSERT`/`UPDATE`/`DELETE` ASTs to emit the `RETURNING ...` SQL.
//!
//! Both submodules are `pub(crate)`; the items that downstream users may
//! need are re-exported from [`crate::query_builder`] under the
//! `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
//! feature, and (for the items consumed by the `table!` macro) from
//! [`crate::internal::table_macro`].
//!
//! [`ReturningQuerySource`]: returning_query_source::ReturningQuerySource
//! [`InsertStmtKind`]: returning_query_source::InsertStmtKind
//! [`ReturningClause<E>`]: returning_clause::ReturningClause
//! [`NoReturningClause`]: returning_clause::NoReturningClause

pub(crate) mod returning_clause;
pub(crate) mod returning_query_source;
