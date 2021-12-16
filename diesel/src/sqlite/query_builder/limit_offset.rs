use crate::query_builder::limit_clause::{LimitClause, NoLimitClause};
use crate::query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use crate::query_builder::offset_clause::{NoOffsetClause, OffsetClause};
use crate::query_builder::{AstPass, IntoBoxedClause, QueryFragment};
use crate::result::QueryResult;
use crate::sqlite::Sqlite;

impl QueryFragment<Sqlite> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        Ok(())
    }
}

impl<L> QueryFragment<Sqlite> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    LimitClause<L>: QueryFragment<Sqlite>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out)?;
        Ok(())
    }
}

impl<O> QueryFragment<Sqlite> for LimitOffsetClause<NoLimitClause, OffsetClause<O>>
where
    OffsetClause<O>: QueryFragment<Sqlite>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        // Sqlite requires a limit clause in front of any offset clause
        // using `LIMIT -1` is the same as not having any limit clause
        // https://sqlite.org/lang_select.html
        out.push_sql(" LIMIT -1 ");
        self.offset_clause.walk_ast(out)?;
        Ok(())
    }
}

impl<L, O> QueryFragment<Sqlite> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    LimitClause<L>: QueryFragment<Sqlite>,
    OffsetClause<O>: QueryFragment<Sqlite>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a> QueryFragment<Sqlite> for BoxedLimitOffsetClause<'a, Sqlite> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        match (self.limit.as_ref(), self.offset.as_ref()) {
            (Some(limit), Some(offset)) => {
                limit.walk_ast(out.reborrow())?;
                offset.walk_ast(out.reborrow())?;
            }
            (Some(limit), None) => {
                limit.walk_ast(out.reborrow())?;
            }
            (None, Some(offset)) => {
                // See the `QueryFragment` implementation for `LimitOffsetClause` for details.
                out.push_sql(" LIMIT -1 ");
                offset.walk_ast(out.reborrow())?;
            }
            (None, None) => {}
        }
        Ok(())
    }
}

// Have explicit impls here because we need to set `Some`/`None` for the clauses
// correspondingly, otherwise we cannot match on it in the `QueryFragment` impl
// above
impl<'a> IntoBoxedClause<'a, Sqlite> for LimitOffsetClause<NoLimitClause, NoOffsetClause> {
    type BoxedClause = BoxedLimitOffsetClause<'a, Sqlite>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: None,
            offset: None,
        }
    }
}

impl<'a, L> IntoBoxedClause<'a, Sqlite> for LimitOffsetClause<LimitClause<L>, NoOffsetClause>
where
    L: QueryFragment<Sqlite> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, Sqlite>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: None,
        }
    }
}

impl<'a, O> IntoBoxedClause<'a, Sqlite> for LimitOffsetClause<NoLimitClause, OffsetClause<O>>
where
    O: QueryFragment<Sqlite> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, Sqlite>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: None,
            offset: Some(Box::new(self.offset_clause)),
        }
    }
}

impl<'a, L, O> IntoBoxedClause<'a, Sqlite> for LimitOffsetClause<LimitClause<L>, OffsetClause<O>>
where
    L: QueryFragment<Sqlite> + Send + 'a,
    O: QueryFragment<Sqlite> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, Sqlite>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: Some(Box::new(self.offset_clause)),
        }
    }
}
