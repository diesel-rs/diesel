use diesel::backend::Backend;
use diesel::expression::expression_types;
use diesel::internal::table_macro::{FromClause, SelectStatement};
use diesel::prelude::*;
use diesel::query_builder::*;
use std::borrow::Borrow;

use crate::column::Column;
use crate::dummy_expression::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// A database table.
/// This type is created by the [`table`](crate::table()) function.
pub struct Table<T, U = T> {
    name: T,
    schema: Option<U>,
}

impl<T, U> Table<T, U> {
    pub(crate) fn new(name: T) -> Self {
        Self { name, schema: None }
    }

    pub(crate) fn with_schema(schema: U, name: T) -> Self {
        Self {
            name,
            schema: Some(schema),
        }
    }

    /// Create a column with this table.
    pub fn column<ST, V>(&self, name: V) -> Column<Self, V, ST>
    where
        Self: Clone,
    {
        Column::new(self.clone(), name)
    }

    /// Gets the name of the table, as specified on creation.
    pub fn name(&self) -> &T {
        &self.name
    }
}

impl<T, U> QuerySource for Table<T, U>
where
    Self: Clone,
{
    type FromClause = Self;
    type DefaultSelection = DummyExpression;

    fn from_clause(&self) -> Self {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        DummyExpression::new()
    }
}

impl<T, U> AsQuery for Table<T, U>
where
    T: Clone,
    U: Clone,
    SelectStatement<FromClause<Self>>: Query<SqlType = expression_types::NotSelectable>,
{
    type SqlType = expression_types::NotSelectable;
    type Query = SelectStatement<FromClause<Self>>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<T, U> diesel::query_source::View for Table<T, U>
where
    Self: QuerySource + AsQuery,
{
    type AllColumns = DummyExpression;
    fn all_columns() -> Self::AllColumns {
        DummyExpression::new()
    }
}

impl<T, U> diesel::Table for Table<T, U>
where
    Self: QuerySource + AsQuery,
{
    type PrimaryKey = DummyExpression;

    fn primary_key(&self) -> Self::PrimaryKey {
        DummyExpression::new()
    }
}

impl<T, U, DB> QueryFragment<DB> for Table<T, U>
where
    DB: Backend,
    T: Borrow<str>,
    U: Borrow<str>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        if let Some(ref schema) = self.schema {
            out.push_identifier(schema.borrow())?;
            out.push_sql(".");
        }

        out.push_identifier(self.name.borrow())?;
        Ok(())
    }
}

impl<T, U> QueryId for Table<T, U> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}
