use super::QueryFragment;
use crate::query_builder::QueryId;

/// A helper query node that contains both limit and offset clauses
///
/// This type is only relevant for implementing custom backends
#[derive(Debug, Clone, Copy, QueryId)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub struct LimitOffsetClause<Limit, Offset> {
    /// The limit clause
    pub limit_clause: Limit,
    /// The offset clause
    pub offset_clause: Offset,
}

/// A boxed variant of [`LimitOffsetClause`](LimitOffsetClause)
///
/// This type is only relevant for implementing custom backends
#[allow(missing_debug_implementations)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub struct BoxedLimitOffsetClause<'a, DB> {
    /// The limit clause
    pub limit: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
    /// The offset clause
    pub offset: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
}
