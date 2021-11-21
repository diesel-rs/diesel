use crate::mysql::Mysql;
use crate::query_builder::limit_clause::{LimitClause, NoLimitClause};
use crate::query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use crate::query_builder::offset_clause::{NoOffsetClause, OffsetClause};
use crate::query_builder::{AstPass, IntoBoxedClause, QueryFragment};
use crate::result::QueryResult;

impl QueryFragment<Mysql> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        Ok(())
    }
}

impl<L> QueryFragment<Mysql> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    LimitClause<L>: QueryFragment<Mysql>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out)?;
        Ok(())
    }
}

impl<L, O> QueryFragment<Mysql> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    LimitClause<L>: QueryFragment<Mysql>,
    OffsetClause<O>: QueryFragment<Mysql>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a> QueryFragment<Mysql> for BoxedLimitOffsetClause<'a, Mysql> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        match (self.limit.as_ref(), self.offset.as_ref()) {
            (Some(limit), Some(offset)) => {
                limit.walk_ast(out.reborrow())?;
                offset.walk_ast(out.reborrow())?;
            }
            (Some(limit), None) => {
                limit.walk_ast(out.reborrow())?;
            }
            (None, Some(offset)) => {
                // Mysql requires a limit clause in front of any offset clause
                // The documentation proposes the following:
                // > To retrieve all rows from a certain offset up to the end of the
                // > result set, you can use some large number for the second parameter.
                // https://dev.mysql.com/doc/refman/8.0/en/select.html
                // Therefore we just use u64::MAX as limit here
                // That does not result in any limitations because mysql only supports
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

impl<'a> IntoBoxedClause<'a, Mysql> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    type BoxedClause = BoxedLimitOffsetClause<'a, Mysql>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: None,
            offset: None,
        }
    }
}

impl<'a, L> IntoBoxedClause<'a, Mysql> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    L: QueryFragment<Mysql> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, Mysql>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: None,
        }
    }
}

impl<'a, L, O> IntoBoxedClause<'a, Mysql> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    L: QueryFragment<Mysql> + Send + 'a,
    O: QueryFragment<Mysql> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, Mysql>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: Some(Box::new(self.offset_clause)),
        }
    }
}
