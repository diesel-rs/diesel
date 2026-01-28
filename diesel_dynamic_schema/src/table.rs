use diesel::backend::Backend;
use diesel::expression::expression_types;
use diesel::internal::table_macro::{FromClause, SelectStatement};
use diesel::prelude::*;
use diesel::query_builder::*;
use std::borrow::Borrow;

use crate::column::Column;
use crate::dummy_expression::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// A database table.
/// This type is created by the [`table`](crate::table()) function.
pub struct Table<T, U = T> {
    name: T,
    schema: Option<U>,
}

impl<'a> From<&'a str> for Table<&'a str> {
    fn from(name: &'a str) -> Self {
        Table::new(name)
    }
}

impl From<String> for Table<String> {
    fn from(name: String) -> Self {
        Table::new(name)
    }
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel_dynamic_schema::{Table, Schema};
    /// let table: Table<&str> = "users".into();
    /// assert_eq!(table.name(), &"users");
    /// let schema: Schema<&str> = "public".into();
    /// let table_with_schema: Table<&str> = schema.table("posts");
    /// assert_eq!(table_with_schema.name(), &"posts");
    /// ```
    pub fn name(&self) -> &T {
        &self.name
    }

    /// Gets the schema of the table, if one was specified on creation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel_dynamic_schema::{Table, Schema};
    /// let schema: Schema<&str> = "public".into();
    /// let table: Table<&str> = schema.table("users");
    /// assert_eq!(table.schema(), Some(&"public"));
    /// let schema_less_table: Table<&str> = "posts".into();
    /// assert_eq!(schema_less_table.schema(), None);
    /// ```
    pub fn schema(&self) -> Option<&U> {
        self.schema.as_ref()
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

impl<T, U> diesel::Table for Table<T, U>
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
