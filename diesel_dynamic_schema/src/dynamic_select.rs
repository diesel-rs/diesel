use diesel::backend::Backend;
use diesel::expression::{is_aggregate, NonAggregate, ValidGrouping};
use diesel::query_builder::{
    AstPass, IntoBoxedSelectClause, QueryFragment, QueryId, SelectClauseExpression,
    SelectClauseQueryFragment,
};
use diesel::sql_types::Untyped;
use diesel::{AppearsOnTable, Expression, QueryResult, SelectableExpression};
use std::marker::PhantomData;

/// Represents a dynamically sized select clause
#[allow(missing_debug_implementations)]
pub struct DynamicSelectClause<'a, DB, QS> {
    selects: Vec<Box<dyn QueryFragment<DB> + Send + 'a>>,
    p: PhantomData<QS>,
}

impl<'a, DB, QS> QueryId for DynamicSelectClause<'a, DB, QS> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<'a, DB, QS> Default for DynamicSelectClause<'a, DB, QS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, DB, QS> DynamicSelectClause<'a, DB, QS> {
    /// Constructs a new dynamically sized select clause without any fields
    pub fn new() -> Self {
        Self {
            selects: Vec::new(),
            p: PhantomData,
        }
    }

    /// Adds the field to the dynamically sized select clause
    pub fn add_field<F>(&mut self, field: F)
    where
        F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
        DB: Backend,
    {
        self.selects.push(Box::new(field))
    }
}

impl<'a, DB, QS> AppearsOnTable<QS> for DynamicSelectClause<'a, DB, QS> where Self: Expression {}

impl<'a, DB, QS> SelectableExpression<QS> for DynamicSelectClause<'a, DB, QS> where
    Self: AppearsOnTable<QS>
{
}

impl<'a, QS, DB> Expression for DynamicSelectClause<'a, DB, QS> {
    type SqlType = Untyped;
}

impl<'a, QS, DB> SelectClauseQueryFragment<QS, DB> for DynamicSelectClause<'a, QS, DB>
where
    DB: Backend,
    Self: QueryFragment<DB>,
{
    fn walk_ast(&self, _source: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB>>::walk_ast(self, pass)
    }
}

impl<'a, DB, QS> QueryFragment<DB> for DynamicSelectClause<'a, DB, QS>
where
    DB: Backend,
{
    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        let mut first = true;
        for s in &self.selects {
            if first {
                first = false;
            } else {
                pass.push_sql(", ");
            }
            s.walk_ast(pass.reborrow())?;
        }
        Ok(())
    }
}

impl<'a, DB, QS> IntoBoxedSelectClause<'a, DB, QS> for DynamicSelectClause<'a, DB, QS>
where
    QS: Send,
    Self: 'a + QueryFragment<DB> + SelectClauseExpression<QS>,
    DB: Backend,
{
    type SqlType = Untyped;

    fn into_boxed(self, _source: &QS) -> Box<dyn QueryFragment<DB> + Send + 'a> {
        Box::new(self)
    }
}

impl<'a, DB, QS> ValidGrouping<()> for DynamicSelectClause<'a, DB, QS> {
    type IsAggregate = is_aggregate::No;
}
