//! Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
//! with or without `ALL` rule for duplicates

use crate::backend::Backend;
use crate::expression::NonAggregate;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::{Insertable, QueryResult, RunQueryDsl, Table};

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoCombinationClause;

impl<DB: Backend> QueryFragment<DB> for NoCombinationClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
/// with or without `ALL` rule for duplicates
pub struct CombinationClause<Combinator, Rule, Source, Rhs> {
    combinator: Combinator,
    duplicate_rule: Rule,
    source: Source,
    rhs: Rhs,
}

impl<Combinator, Rule, Source, Rhs> CombinationClause<Combinator, Rule, Source, Rhs> {
    /// Create a new combination
    pub(crate) fn new(
        combinator: Combinator,
        duplicate_rule: Rule,
        source: Source,
        rhs: Rhs,
    ) -> Self {
        CombinationClause {
            combinator,
            duplicate_rule,
            source,
            rhs,
        }
    }
}

impl<Combinator, Rule, Source, Rhs> Query for CombinationClause<Combinator, Rule, Source, Rhs>
where
    Source: Query,
    Rhs: Query<SqlType = Source::SqlType>,
{
    type SqlType = Source::SqlType;
}

impl<Combinator, Rule, Source, Rhs, Conn> RunQueryDsl<Conn>
    for CombinationClause<Combinator, Rule, Source, Rhs>
{
}

impl<Combinator, Rule, Source, Rhs, T> Insertable<T>
    for CombinationClause<Combinator, Rule, Source, Rhs>
where
    T: Table,
    T::AllColumns: NonAggregate,
    Self: Query,
{
    type Values = InsertFromSelect<Self, T::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Computes the set union of the rows returned by the involved `SELECT` statements using SQL `UNION`
pub struct Union;

impl<DB: Backend> QueryFragment<DB> for Union {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" UNION ");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Computes the set intersection of the rows returned by the involved `SELECT` statements using SQL `INTERSECT`
pub struct Intersect;

impl<DB: Backend> QueryFragment<DB> for Intersect {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" INTERSECT ");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Computes the set difference of the rows returned by the involved `SELECT` statements using SQL `EXCEPT`
pub struct Except;

impl<DB: Backend> QueryFragment<DB> for Except {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" EXCEPT ");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Remove duplicate rows in the result, this is the default behavior of `UNION`, `INTERSECT` and `EXCEPT`
pub struct Distinct;

impl<DB: Backend> QueryFragment<DB> for Distinct {
    fn walk_ast(&self, _out: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Keep duplicate rows in the result
pub struct All;

impl<DB: Backend> QueryFragment<DB> for All {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("ALL ");
        Ok(())
    }
}

/// Marker trait used to indicate whenever the combination is supported by given backend
pub trait CombinationSupportedBy<DB> {}

impl<Source, Rhs, DB> CombinationSupportedBy<DB>
    for CombinationClause<Union, Distinct, Source, Rhs>
{
}
impl<Source, Rhs, DB> CombinationSupportedBy<DB> for CombinationClause<Union, All, Source, Rhs> {}

#[cfg(feature = "postgres")]
mod postgres {
    use super::*;
    use crate::pg::Pg;
    use crate::query_builder::{AstPass, QueryFragment};
    use crate::QueryResult;

    impl<Source, Rhs> CombinationSupportedBy<Pg>
        for CombinationClause<Intersect, Distinct, Source, Rhs>
    {
    }
    impl<Source, Rhs> CombinationSupportedBy<Pg> for CombinationClause<Intersect, All, Source, Rhs> {}
    impl<Source, Rhs> CombinationSupportedBy<Pg> for CombinationClause<Except, Distinct, Source, Rhs> {}
    impl<Source, Rhs> CombinationSupportedBy<Pg> for CombinationClause<Except, All, Source, Rhs> {}

    impl<Combinator, Rule, Source, Rhs> QueryFragment<Pg>
        for CombinationClause<Combinator, Rule, Source, Rhs>
    where
        Combinator: QueryFragment<Pg>,
        Rule: QueryFragment<Pg>,
        Source: QueryFragment<Pg>,
        Rhs: QueryFragment<Pg>,
        Self: CombinationSupportedBy<Pg>,
    {
        fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
            self.source.walk_ast(out.reborrow())?;
            self.combinator.walk_ast(out.reborrow())?;
            self.duplicate_rule.walk_ast(out.reborrow())?;
            out.push_sql("(");
            self.rhs.walk_ast(out.reborrow())?;
            out.push_sql(")");
            Ok(())
        }
    }
}

#[cfg(feature = "mysql")]
mod mysql {
    use super::*;
    use crate::mysql::Mysql;
    use crate::query_builder::{AstPass, QueryFragment};
    use crate::QueryResult;

    impl<Combinator, Rule, Source, Rhs> QueryFragment<Mysql>
        for CombinationClause<Combinator, Rule, Source, Rhs>
    where
        Combinator: QueryFragment<Mysql>,
        Rule: QueryFragment<Mysql>,
        Source: QueryFragment<Mysql>,
        Rhs: QueryFragment<Mysql>,
        Self: CombinationSupportedBy<Mysql>,
    {
        fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
            self.source.walk_ast(out.reborrow())?;
            self.combinator.walk_ast(out.reborrow())?;
            self.duplicate_rule.walk_ast(out.reborrow())?;
            out.push_sql("(");
            self.rhs.walk_ast(out.reborrow())?;
            out.push_sql(")");
            Ok(())
        }
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use crate::query_builder::{AstPass, QueryFragment};
    use crate::sqlite::Sqlite;
    use crate::QueryResult;

    impl<Source, Rhs> CombinationSupportedBy<Sqlite>
        for CombinationClause<Intersect, Distinct, Source, Rhs>
    {
    }
    impl<Source, Rhs> CombinationSupportedBy<Sqlite>
        for CombinationClause<Except, Distinct, Source, Rhs>
    {
    }

    impl<Combinator, Rule, Source, Rhs> QueryFragment<Sqlite>
        for CombinationClause<Combinator, Rule, Source, Rhs>
    where
        Combinator: QueryFragment<Sqlite>,
        Rule: QueryFragment<Sqlite>,
        Source: QueryFragment<Sqlite>,
        Rhs: QueryFragment<Sqlite>,
        Self: CombinationSupportedBy<Sqlite>,
    {
        fn walk_ast(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
            self.source.walk_ast(out.reborrow())?;
            self.combinator.walk_ast(out.reborrow())?;
            self.duplicate_rule.walk_ast(out.reborrow())?;
            self.rhs.walk_ast(out.reborrow())?;
            Ok(())
        }
    }
}
