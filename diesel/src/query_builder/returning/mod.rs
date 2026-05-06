//! Helpers for type-checking and generating SQL `RETURNING` clauses.
//!
//! This module is split in two:
//!
//! * [`returning_expression`] — the [`ReturningExpression`] trait that drives
//!   how `INSERT`/`UPDATE`/`DELETE` `RETURNING` lists are type-checked, plus
//!   the statement-kind markers (`UpdateStmt`, `InsertStmt`, `DeleteStmt`,
//!   `InsertOnConflictDoUpdateStmt`) used as its `Stmt` parameter.
//!
//! * [`returning_clause`] — the actual query-fragment types
//!   ([`ReturningClause<E>`] and [`NoReturningClause`]) that get embedded
//!   inside the `INSERT`/`UPDATE`/`DELETE` ASTs to emit the `RETURNING ...`
//!   SQL. This submodule is only re-exported when the
//!   `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
//!   feature is enabled.
//!
//! [`ReturningExpression`]: returning_expression::ReturningExpression
//! [`ReturningClause<E>`]: returning_clause::ReturningClause
//! [`NoReturningClause`]: returning_clause::NoReturningClause

pub mod returning_expression;

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
pub(crate) mod returning_clause;
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub mod returning_clause;
