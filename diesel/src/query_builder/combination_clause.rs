//! Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
//! with or without `ALL` rule for duplicates

use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::subselect::ValidSubselect;
use crate::expression::NonAggregate;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::{AsQuery, AstPass, Query, QueryFragment, QueryId, SelectQuery};
use crate::{CombineDsl, Insertable, QueryResult, RunQueryDsl, Table};

#[derive(Debug, Clone, Copy, QueryId)]
pub(crate) struct NoCombinationClause;

impl<DB> QueryFragment<DB> for NoCombinationClause
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
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
    source: ParenthesisWrapper<Source>,
    rhs: ParenthesisWrapper<Rhs>,
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
            source: ParenthesisWrapper(source),
            rhs: ParenthesisWrapper(rhs),
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

impl<Combinator, Rule, Source, Rhs> SelectQuery for CombinationClause<Combinator, Rule, Source, Rhs>
where
    Source: SelectQuery,
    Rhs: SelectQuery<SqlType = Source::SqlType>,
{
    type SqlType = Source::SqlType;
}

impl<Combinator, Rule, Source, Rhs, QS> ValidSubselect<QS>
    for CombinationClause<Combinator, Rule, Source, Rhs>
where
    Source: ValidSubselect<QS>,
    Rhs: ValidSubselect<QS>,
{
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

impl<Combinator, Rule, Source, OriginRhs> CombineDsl
    for CombinationClause<Combinator, Rule, Source, OriginRhs>
where
    Self: Query,
{
    type Query = Self;

    fn union<Rhs>(self, rhs: Rhs) -> crate::dsl::Union<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Union, Distinct, self, rhs.as_query())
    }

    fn union_all<Rhs>(self, rhs: Rhs) -> crate::dsl::UnionAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Union, All, self, rhs.as_query())
    }

    fn intersect<Rhs>(self, rhs: Rhs) -> crate::dsl::Intersect<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Intersect, Distinct, self, rhs.as_query())
    }

    fn intersect_all<Rhs>(self, rhs: Rhs) -> crate::dsl::IntersectAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Intersect, All, self, rhs.as_query())
    }

    fn except<Rhs>(self, rhs: Rhs) -> crate::dsl::Except<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Except, Distinct, self, rhs.as_query())
    }

    fn except_all<Rhs>(self, rhs: Rhs) -> crate::dsl::ExceptAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Except, All, self, rhs.as_query())
    }
}

impl<Combinator, Rule, Source, Rhs, DB: Backend> QueryFragment<DB>
    for CombinationClause<Combinator, Rule, Source, Rhs>
where
    Combinator: QueryFragment<DB>,
    Rule: QueryFragment<DB>,
    ParenthesisWrapper<Source>: QueryFragment<DB>,
    ParenthesisWrapper<Rhs>: QueryFragment<DB>,
    DB: Backend + SupportsCombinationClause<Combinator, Rule> + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.source.walk_ast(out.reborrow())?;
        self.combinator.walk_ast(out.reborrow())?;
        self.duplicate_rule.walk_ast(out.reborrow())?;
        self.rhs.walk_ast(out)
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Computes the set union of the rows returned by the involved `SELECT` statements using SQL `UNION`
pub struct Union;

impl<DB> QueryFragment<DB> for Union
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" UNION ");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Computes the set intersection of the rows returned by the involved `SELECT` statements using SQL `INTERSECT`
pub struct Intersect;

impl<DB> QueryFragment<DB> for Intersect
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" INTERSECT ");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Computes the set difference of the rows returned by the involved `SELECT` statements using SQL `EXCEPT`
pub struct Except;

impl<DB> QueryFragment<DB> for Except
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" EXCEPT ");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Remove duplicate rows in the result, this is the default behavior of `UNION`, `INTERSECT` and `EXCEPT`
pub struct Distinct;

impl<DB> QueryFragment<DB> for Distinct
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
/// Keep duplicate rows in the result
pub struct All;

impl<DB> QueryFragment<DB> for All
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("ALL ");
        Ok(())
    }
}

/// Marker trait used to indicate whenever a backend supports given combination
pub trait SupportsCombinationClause<Combinator, Rule> {}

#[derive(Debug, Copy, Clone, QueryId)]
/// Wrapper used to wrap rhs sql in parenthesis when supported by backend
pub struct ParenthesisWrapper<T>(T);

#[cfg(feature = "postgres_backend")]
mod postgres {
    use super::*;
    use crate::pg::Pg;

    impl<T: QueryFragment<Pg>> QueryFragment<Pg> for ParenthesisWrapper<T> {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
            out.push_sql("(");
            self.0.walk_ast(out.reborrow())?;
            out.push_sql(")");
            Ok(())
        }
    }

    impl SupportsCombinationClause<Union, Distinct> for Pg {}
    impl SupportsCombinationClause<Union, All> for Pg {}
    impl SupportsCombinationClause<Intersect, Distinct> for Pg {}
    impl SupportsCombinationClause<Intersect, All> for Pg {}
    impl SupportsCombinationClause<Except, Distinct> for Pg {}
    impl SupportsCombinationClause<Except, All> for Pg {}
}

#[cfg(feature = "mysql_backend")]
mod mysql {
    use super::*;
    use crate::mysql::Mysql;

    impl<T: QueryFragment<Mysql>> QueryFragment<Mysql> for ParenthesisWrapper<T> {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
            out.push_sql("(");
            self.0.walk_ast(out.reborrow())?;
            out.push_sql(")");
            Ok(())
        }
    }

    impl SupportsCombinationClause<Union, Distinct> for Mysql {}
    impl SupportsCombinationClause<Union, All> for Mysql {}
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use crate::sqlite::Sqlite;

    impl<T: QueryFragment<Sqlite>> QueryFragment<Sqlite> for ParenthesisWrapper<T> {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
            // SQLite does not support parenthesis around this clause
            // we can emulate this by construct a fake outer
            // SELECT * FROM (inner_query) statement
            out.push_sql("SELECT * FROM (");
            self.0.walk_ast(out.reborrow())?;
            out.push_sql(")");
            Ok(())
        }
    }

    impl SupportsCombinationClause<Union, Distinct> for Sqlite {}
    impl SupportsCombinationClause<Union, All> for Sqlite {}
    impl SupportsCombinationClause<Intersect, Distinct> for Sqlite {}
    impl SupportsCombinationClause<Except, Distinct> for Sqlite {}
}
