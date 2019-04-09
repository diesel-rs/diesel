use mysql::Mysql;
use query_builder::limit_clause::{LimitClause, NoLimitClause};
use query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use query_builder::offset_clause::{NoOffsetClause, OffsetClause};
use query_builder::{AstPass, QueryFragment};
use result::QueryResult;

impl QueryFragment<Mysql> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    fn walk_ast(&self, _out: AstPass<Mysql>) -> QueryResult<()> {
        Ok(())
    }
}

impl<L> QueryFragment<Mysql> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    LimitClause<L>: QueryFragment<Mysql>,
{
    fn walk_ast(&self, out: AstPass<Mysql>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out)?;
        Ok(())
    }
}

impl<L, O> QueryFragment<Mysql> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    LimitClause<L>: QueryFragment<Mysql>,
    OffsetClause<O>: QueryFragment<Mysql>,
{
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a> QueryFragment<Mysql> for BoxedLimitOffsetClause<'a, Mysql> {
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
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

impl<'a> From<LimitOffsetClause<NoLimitClause, NoOffsetClause>>
    for BoxedLimitOffsetClause<'a, Mysql>
{
    fn from(_limit_offset: LimitOffsetClause<NoLimitClause, NoOffsetClause>) -> Self {
        Self {
            limit: None,
            offset: None,
        }
    }
}

impl<'a, L> From<LimitOffsetClause<LimitClause<L>, NoOffsetClause>>
    for BoxedLimitOffsetClause<'a, Mysql>
where
    L: QueryFragment<Mysql> + 'a,
{
    fn from(limit_offset: LimitOffsetClause<LimitClause<L>, NoOffsetClause>) -> Self {
        Self {
            limit: Some(Box::new(limit_offset.limit_clause)),
            offset: None,
        }
    }
}

impl<'a, L, O> From<LimitOffsetClause<LimitClause<L>, OffsetClause<O>>>
    for BoxedLimitOffsetClause<'a, Mysql>
where
    L: QueryFragment<Mysql> + 'a,
    O: QueryFragment<Mysql> + 'a,
{
    fn from(limit_offset: LimitOffsetClause<LimitClause<L>, OffsetClause<O>>) -> Self {
        Self {
            limit: Some(Box::new(limit_offset.limit_clause)),
            offset: Some(Box::new(limit_offset.offset_clause)),
        }
    }
}
