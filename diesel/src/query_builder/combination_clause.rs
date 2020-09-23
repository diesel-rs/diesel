//! Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
//! with or without `ALL` rule for duplicates

use crate::backend::Backend;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::QueryDsl;
use crate::{Insertable, QueryResult};
use crate::{RunQueryDsl, Table};
use std::marker::Sized;

#[derive(Debug, Copy, Clone, QueryId)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// Combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
/// with or without `ALL` rule for duplicates
pub struct Combination<Combinator, Rule, Source, Rhs> {
    combinator: Combinator,
    duplicate_rule: Rule,
    source: Source,
    rhs: Rhs,
}

impl<Combinator, Rule, Source, Rhs> Combination<Combinator, Rule, Source, Rhs> {
    /// Create a new combination
    pub fn new(combinator: Combinator, duplicate_rule: Rule, source: Source, rhs: Rhs) -> Self {
        Combination {
            combinator,
            duplicate_rule,
            source,
            rhs,
        }
    }
}

impl<Combinator, Rule, Source, Rhs> Query for Combination<Combinator, Rule, Source, Rhs>
where
    Source: Query,
    Rhs: Query<SqlType = Source::SqlType>,
{
    type SqlType = Source::SqlType;
}

impl<Combinator, Rule, Source, Rhs, DB> QueryFragment<DB>
    for Combination<Combinator, Rule, Source, Rhs>
where
    Combinator: QueryFragment<DB>,
    Rule: QueryFragment<DB>,
    Source: QueryFragment<DB>,
    Rhs: QueryFragment<DB>,
    DB: Backend,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.source.walk_ast(out.reborrow())?;
        self.combinator.walk_ast(out.reborrow())?;
        self.duplicate_rule.walk_ast(out.reborrow())?;
        self.rhs.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Combinator, Rule, Source, Rhs, Conn> RunQueryDsl<Conn>
    for Combination<Combinator, Rule, Source, Rhs>
{
}

impl<Combinator, Rule, Source, Rhs> QueryDsl for Combination<Combinator, Rule, Source, Rhs> {}

impl<Combinator, Rule, Source, Rhs, T> Insertable<T> for Combination<Combinator, Rule, Source, Rhs>
where
    T: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, T::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

/// Extension trait to combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
/// with or without `ALL` rule for duplicates
pub trait SelectCombinationQueryMethods: Query + Sized {
    /// Combine two queries using a SQL `UNION`
    ///
    /// # Examples
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, animals};
    ///
    /// # fn main() {
    /// #     use self::users::dsl::{users, name as user_name};
    /// #     use self::animals::dsl::{animals, name as animal_name};
    /// #     use diesel::query_builder::combination_clause::SelectCombinationQueryMethods;
    /// #     let connection = establish_connection();
    /// let data = users.select(user_name.nullable())
    ///     .union(animals.select(animal_name))
    ///     .load(&connection);
    ///
    /// let expected_data = vec![
    ///     None,
    ///     Some(String::from("Jack")),
    ///     Some(String::from("Tess")),
    ///     Some(String::from("Sean")),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    fn union<Rhs>(self, rhs: Rhs) -> Combination<Union, Distinct, Self, Rhs>
    where
        Rhs: Query<SqlType = Self::SqlType>,
    {
        Combination::new(Union, Distinct, self, rhs)
    }

    /// Combine two queries using a SQL `UNION ALL`
    fn union_all<Rhs>(self, rhs: Rhs) -> Combination<Union, All, Self, Rhs>
    where
        Rhs: Query<SqlType = Self::SqlType>,
    {
        Combination::new(Union, All, self, rhs)
    }

    /// Combine two queries using a SQL `INTERSECT`
    fn intersect<Rhs>(self, rhs: Rhs) -> Combination<Intersect, Distinct, Self, Rhs>
    where
        Rhs: Query<SqlType = Self::SqlType>,
    {
        Combination::new(Intersect, Distinct, self, rhs)
    }

    /// Combine two queries using a SQL `INTERSECT ALL`
    fn intersect_all<Rhs>(self, rhs: Rhs) -> Combination<Intersect, All, Self, Rhs>
    where
        Rhs: Query<SqlType = Self::SqlType>,
    {
        Combination::new(Intersect, All, self, rhs)
    }

    /// Combine two queries using a SQL `EXCEPT`
    fn except<Rhs>(self, rhs: Rhs) -> Combination<Except, Distinct, Self, Rhs>
    where
        Rhs: Query<SqlType = Self::SqlType>,
    {
        Combination::new(Except, Distinct, self, rhs)
    }

    /// Combine two queries using a SQL `EXCEPT ALL`
    fn except_all<Rhs>(self, rhs: Rhs) -> Combination<Except, All, Self, Rhs>
    where
        Rhs: Query<SqlType = Self::SqlType>,
    {
        Combination::new(Except, All, self, rhs)
    }
}

impl<T: Query> SelectCombinationQueryMethods for T {}

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
