//! Types involved in type-checking and emitting SQL `RETURNING` clauses.

pub(crate) mod returning_clause;
pub(crate) mod returning_query_source;

#[doc(inline)]
pub use self::returning_clause::*;
#[doc(inline)]
pub use self::returning_query_source::*;
