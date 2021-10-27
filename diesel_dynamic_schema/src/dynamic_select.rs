use diesel::backend::Backend;
use diesel::expression::{is_aggregate, NonAggregate, ValidGrouping};
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
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

impl<'a, DB, QS> QueryFragment<DB> for DynamicSelectClause<'a, DB, QS>
where
    DB: Backend,
{
    fn walk_ast<'b: 'c, 'c>(&'b self, mut pass: AstPass<'_, 'c, DB>) -> QueryResult<()> {
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

impl<'a, DB, QS> ValidGrouping<()> for DynamicSelectClause<'a, DB, QS> {
    type IsAggregate = is_aggregate::No;
}
