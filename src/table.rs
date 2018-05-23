use diesel::backend::Backend;
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_source::QuerySource;
use diesel;
use std::borrow::Borrow;

use column::Column;
use dummy_expression::*;

#[derive(Debug, Clone, Copy)]
pub struct Table<T> {
    name: T,
}

impl<T> Table<T> {
    pub(crate) fn new(name: T) -> Self {
        Self { name }
    }

    pub fn column<ST, U>(&self, name: U) -> Column<Self, U, ST>
    where
        Self: Clone,
    {
        Column::new(self.clone(), name)
    }
}

impl<T> QuerySource for Table<T>
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

impl<T> AsQuery for Table<T>
where
    SelectStatement<Self>: Query<SqlType = ()>,
{
    type SqlType = ();
    type Query = SelectStatement<Self>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<T> diesel::Table for Table<T>
where
    Self: QuerySource + AsQuery,
{
    type PrimaryKey = DummyExpression;
    type AllColumns = DummyExpression;

    fn primary_key(&self) -> Self::PrimaryKey {
        DummyExpression::new()
    }

    fn all_columns() -> Self::AllColumns {
        DummyExpression::new()
    }
}

impl<T, DB> QueryFragment<DB> for Table<T>
where
    DB: Backend,
    T: Borrow<str>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        out.push_identifier(self.name.borrow())?;
        Ok(())
    }
}

impl<T> QueryId for Table<T> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}
