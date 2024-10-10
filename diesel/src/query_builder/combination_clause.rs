//! Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
//! with or without `ALL` rule for duplicates
//!
//! Within this module, types commonly use the following abbreviations:
//!
//! O: Order By Clause
//! L: Limit Clause
//! Of: Offset Clause
//! LOf: Limit Offset Clause

use crate::backend::{Backend, DieselReserveSpecialization};
use crate::dsl::AsExprOf;
use crate::expression::subselect::ValidSubselect;
use crate::expression::IntoSql;
use crate::expression::NonAggregate;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::limit_clause::{LimitClause, NoLimitClause};
use crate::query_builder::limit_offset_clause::LimitOffsetClause;
use crate::query_builder::offset_clause::{NoOffsetClause, OffsetClause};
use crate::query_builder::order_clause::{NoOrderClause, OrderClause};
use crate::query_builder::{AsQuery, AstPass, Query, QueryFragment, QueryId, SelectQuery};
use crate::query_dsl::methods::*;
use crate::query_dsl::positional_order_dsl::{IntoPositionalOrderExpr, PositionalOrderDsl};
use crate::sql_types::BigInt;
use crate::{CombineDsl, Insertable, QueryDsl, QueryResult, RunQueryDsl, Table};

#[derive(Debug, Copy, Clone, QueryId)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
/// with or without `ALL` rule for duplicates
pub struct CombinationClause<
    Combinator,
    Rule,
    Source,
    Rhs,
    Order = NoOrderClause,
    LimitOffset = LimitOffsetClause<NoLimitClause, NoOffsetClause>,
> {
    combinator: Combinator,
    duplicate_rule: Rule,
    source: ParenthesisWrapper<Source>,
    rhs: ParenthesisWrapper<Rhs>,
    /// The order clause of the query
    order: Order,
    /// The combined limit/offset clause of the query
    limit_offset: LimitOffset,
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
            source: ParenthesisWrapper { inner: source },
            rhs: ParenthesisWrapper { inner: rhs },
            order: NoOrderClause,
            limit_offset: LimitOffsetClause {
                limit_clause: NoLimitClause,
                offset_clause: NoOffsetClause,
            },
        }
    }
}

impl<Combinator, Rule, Source, Rhs, O, LOf> QueryDsl
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
{
}

impl<Combinator, Rule, Source, Rhs, O, LOf> Query
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
where
    Source: Query,
    Rhs: Query<SqlType = Source::SqlType>,
{
    type SqlType = Source::SqlType;
}

impl<Combinator, Rule, Source, Rhs, O, LOf> SelectQuery
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
where
    Source: SelectQuery,
    Rhs: SelectQuery<SqlType = Source::SqlType>,
{
    type SqlType = Source::SqlType;
}

impl<Combinator, Rule, Source, Rhs, O, LOf, QS> ValidSubselect<QS>
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
where
    Source: ValidSubselect<QS>,
    Rhs: ValidSubselect<QS>,
{
}

impl<Combinator, Rule, Source, Rhs, O, LOf, Conn> RunQueryDsl<Conn>
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
{
}

impl<Combinator, Rule, Source, Rhs, O, LOf, T> Insertable<T>
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
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

impl<Combinator, Rule, Source, OriginRhs, O, LOf> CombineDsl
    for CombinationClause<Combinator, Rule, Source, OriginRhs, O, LOf>
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

impl<Combinator, Rule, Source, Rhs, O, LOf, DB: Backend> QueryFragment<DB>
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
where
    Combinator: QueryFragment<DB>,
    Rule: QueryFragment<DB>,
    ParenthesisWrapper<Source>: QueryFragment<DB>,
    ParenthesisWrapper<Rhs>: QueryFragment<DB>,
    O: QueryFragment<DB>,
    LOf: QueryFragment<DB>,
    DB: Backend + SupportsCombinationClause<Combinator, Rule> + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.source.walk_ast(out.reborrow())?;
        self.combinator.walk_ast(out.reborrow())?;
        self.duplicate_rule.walk_ast(out.reborrow())?;
        self.rhs.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit_offset.walk_ast(out)
    }
}

impl<ST, Combinator, Rule, Source, Rhs, O, LOf, RawExpr, Expr> PositionalOrderDsl<RawExpr>
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LOf>
where
    Self: SelectQuery<SqlType = ST>,
    CombinationClause<Combinator, Rule, Source, Rhs, OrderClause<Expr>, LOf>:
        SelectQuery<SqlType = ST>,
    RawExpr: IntoPositionalOrderExpr<Output = Expr>,
{
    type Output = CombinationClause<Combinator, Rule, Source, Rhs, OrderClause<Expr>, LOf>;

    fn positional_order_by(self, expr: RawExpr) -> Self::Output {
        let order = OrderClause(expr.into_positional_expr());

        CombinationClause {
            combinator: self.combinator,
            duplicate_rule: self.duplicate_rule,
            source: self.source,
            rhs: self.rhs,
            order,
            limit_offset: self.limit_offset,
        }
    }
}

#[doc(hidden)]
type Limit = AsExprOf<i64, BigInt>;

impl<ST, Combinator, Rule, Source, Rhs, O, L, Of> LimitDsl
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LimitOffsetClause<L, Of>>
where
    Self: SelectQuery<SqlType = ST>,
    CombinationClause<Combinator, Rule, Source, Rhs, O, LimitOffsetClause<LimitClause<Limit>, Of>>:
        SelectQuery<SqlType = ST>,
{
    type Output = CombinationClause<
        Combinator,
        Rule,
        Source,
        Rhs,
        O,
        LimitOffsetClause<LimitClause<Limit>, Of>,
    >;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(limit.into_sql::<BigInt>());
        CombinationClause {
            combinator: self.combinator,
            duplicate_rule: self.duplicate_rule,
            source: self.source,
            rhs: self.rhs,
            order: self.order,
            limit_offset: LimitOffsetClause {
                limit_clause,
                offset_clause: self.limit_offset.offset_clause,
            },
        }
    }
}

#[doc(hidden)]
type Offset = Limit;

impl<ST, Combinator, Rule, Source, Rhs, O, L, Of> OffsetDsl
    for CombinationClause<Combinator, Rule, Source, Rhs, O, LimitOffsetClause<L, Of>>
where
    Self: SelectQuery<SqlType = ST>,
    CombinationClause<Combinator, Rule, Source, Rhs, O, LimitOffsetClause<L, OffsetClause<Offset>>>:
        SelectQuery<SqlType = ST>,
{
    type Output = CombinationClause<
        Combinator,
        Rule,
        Source,
        Rhs,
        O,
        LimitOffsetClause<L, OffsetClause<Offset>>,
    >;

    fn offset(self, offset: i64) -> Self::Output {
        let offset_clause = OffsetClause(offset.into_sql::<BigInt>());
        CombinationClause {
            combinator: self.combinator,
            duplicate_rule: self.duplicate_rule,
            source: self.source,
            rhs: self.rhs,
            order: self.order,
            limit_offset: LimitOffsetClause {
                limit_clause: self.limit_offset.limit_clause,
                offset_clause,
            },
        }
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
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(inner)
)]
pub struct ParenthesisWrapper<T> {
    /// the inner parenthesis definition
    #[allow(dead_code)]
    inner: T,
}

#[cfg(feature = "postgres_backend")]
mod postgres {
    use super::*;
    use crate::pg::Pg;

    impl<T: QueryFragment<Pg>> QueryFragment<Pg> for ParenthesisWrapper<T> {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
            out.push_sql("(");
            self.inner.walk_ast(out.reborrow())?;
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
            self.inner.walk_ast(out.reborrow())?;
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
            self.inner.walk_ast(out.reborrow())?;
            out.push_sql(")");
            Ok(())
        }
    }

    impl SupportsCombinationClause<Union, Distinct> for Sqlite {}
    impl SupportsCombinationClause<Union, All> for Sqlite {}
    impl SupportsCombinationClause<Intersect, Distinct> for Sqlite {}
    impl SupportsCombinationClause<Except, Distinct> for Sqlite {}
}
