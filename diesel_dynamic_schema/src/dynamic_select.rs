use alloc::boxed::Box;
use alloc::vec::Vec;
use core::iter::FromIterator;
use core::marker::PhantomData;
use diesel::backend::Backend;
use diesel::expression::{is_aggregate, NonAggregate, ValidGrouping};
use diesel::query_builder::{AstPass, OutputFieldMetadata, QueryFragment, QueryId};
use diesel::sql_types::Untyped;
use diesel::{AppearsOnTable, Expression, QueryResult, SelectableExpression};

/// Represents a dynamically sized output clause.
#[allow(missing_debug_implementations)]
pub struct DynamicOutputClause<'a, DB, QS> {
    selects: Vec<Box<dyn QueryFragment<DB> + Send + 'a>>,
    p: PhantomData<QS>,
}

impl<DB, QS> QueryId for DynamicOutputClause<'_, DB, QS> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<DB, QS> Default for DynamicOutputClause<'_, DB, QS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, DB, QS> DynamicOutputClause<'a, DB, QS> {
    /// Constructs a new dynamically sized output clause without any fields.
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

    /// Adds a field and returns the clause.
    pub fn field<F>(mut self, field: F) -> Self
    where
        F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
        DB: Backend,
    {
        self.add_field(field);
        self
    }

    /// Adds multiple fields to the dynamically sized output clause.
    pub fn add_fields<I, F>(&mut self, fields: I)
    where
        I: IntoIterator<Item = F>,
        F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
        DB: Backend,
    {
        for field in fields {
            self.add_field(field);
        }
    }

    /// Returns the number of fields in the select clause
    pub fn len(&self) -> usize {
        self.selects.len()
    }

    /// Returns whether the select clause is empty
    pub fn is_empty(&self) -> bool {
        self.selects.is_empty()
    }
}

impl<DB, QS> AppearsOnTable<QS> for DynamicOutputClause<'_, DB, QS> where Self: Expression {}

impl<DB, QS> SelectableExpression<QS> for DynamicOutputClause<'_, DB, QS> where
    Self: AppearsOnTable<QS>
{
}

impl<QS, DB> Expression for DynamicOutputClause<'_, DB, QS> {
    type SqlType = Untyped;
}

impl<DB, QS> QueryFragment<DB> for DynamicOutputClause<'_, DB, QS>
where
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
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

    fn collect_output_metadata<'b>(
        &'b self,
        out: &mut Vec<OutputFieldMetadata<'b>>,
    ) -> QueryResult<()> {
        for s in &self.selects {
            s.collect_output_metadata(out)?;
        }
        Ok(())
    }
}

impl<DB, QS> ValidGrouping<()> for DynamicOutputClause<'_, DB, QS> {
    type IsAggregate = is_aggregate::No;
}

impl<'a, DB, QS, F> FromIterator<F> for DynamicOutputClause<'a, DB, QS>
where
    F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
    DB: Backend,
{
    fn from_iter<I: IntoIterator<Item = F>>(iter: I) -> Self {
        let mut select_clause = DynamicOutputClause::new();
        select_clause.add_fields(iter);
        select_clause
    }
}

impl<'a, DB, QS, F> core::iter::Extend<F> for DynamicOutputClause<'a, DB, QS>
where
    F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
    DB: Backend,
{
    fn extend<I: IntoIterator<Item = F>>(&mut self, iter: I) {
        self.add_fields(iter)
    }
}
