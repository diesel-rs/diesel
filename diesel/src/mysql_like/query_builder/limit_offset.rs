use crate::mysql_like::MysqlLikeBackend;
//use crate::B::B;
use crate::query_builder::limit_clause::{LimitClause, NoLimitClause};
use crate::query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use crate::query_builder::offset_clause::{NoOffsetClause, OffsetClause};
use crate::query_builder::{AstPass, IntoBoxedClause, QueryFragment};
use crate::result::QueryResult;

impl<B: MysqlLikeBackend> QueryFragment<B> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        Ok(())
    }
}

impl<B: MysqlLikeBackend,L> QueryFragment<B> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    LimitClause<L>: QueryFragment<B>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out)?;
        Ok(())
    }
}

impl<B: MysqlLikeBackend,L, O> QueryFragment<B> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    LimitClause<L>: QueryFragment<B>,
    OffsetClause<O>: QueryFragment<B>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<B: MysqlLikeBackend> QueryFragment<B> for BoxedLimitOffsetClause<'_, B> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        match (self.limit.as_ref(), self.offset.as_ref()) {
            (Some(limit), Some(offset)) => {
                limit.walk_ast(out.reborrow())?;
                offset.walk_ast(out.reborrow())?;
            }
            (Some(limit), None) => {
                limit.walk_ast(out.reborrow())?;
            }
            (None, Some(offset)) => {
                // B requires a limit clause in front of any offset clause
                // The documentation proposes the following:
                // > To retrieve all rows from a certain offset up to the end of the
                // > result set, you can use some large number for the second parameter.
                // https://dev.B.com/doc/refman/8.0/en/select.html
                // Therefore we just use u64::MAX as limit here
                // That does not result in any limitations because B only supports
                // up to 64TB of data per table. Assuming 1 bit per row this means
                // 1024 * 1024 * 1024 * 1024 * 8 = 562.949.953.421.312 rows which is smaller
                // than 2^64 = 18.446.744.073.709.551.615
                out.push_sql(" LIMIT 18446744073709551615 ");
                offset.walk_ast(out.reborrow())?;
            }
            (None, None) => {}
        }
        Ok(())
    }
}

impl<'a, B: MysqlLikeBackend> IntoBoxedClause<'a, B> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    type BoxedClause = BoxedLimitOffsetClause<'a, B>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: None,
            offset: None,
        }
    }
}

impl<'a, B: MysqlLikeBackend, L> IntoBoxedClause<'a, B> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    L: QueryFragment<B> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, B>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: None,
        }
    }
}

impl<'a, B: MysqlLikeBackend, L, O> IntoBoxedClause<'a, B> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    L: QueryFragment<B> + Send + 'a,
    O: QueryFragment<B> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, B>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: Some(Box::new(self.offset_clause)),
        }
    }
}
