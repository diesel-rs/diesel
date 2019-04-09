use super::QueryFragment;
use crate::query_builder::QueryId;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct LimitOffsetClause<Limit, Offset> {
    pub(crate) limit_clause: Limit,
    pub(crate) offset_clause: Offset,
}

#[allow(missing_debug_implementations)]
pub struct BoxedLimitOffsetClause<'a, DB> {
    pub(crate) limit: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
    pub(crate) offset: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
}
