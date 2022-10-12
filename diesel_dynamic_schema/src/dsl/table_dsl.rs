//! This module contains database creation / deletion statements.

use diesel::{
    backend::Backend,
    query_builder::*,
    result::{Error, QueryResult},
    RunQueryDsl,
};

/// Represents a SQL `DROP TABLE` statement.
#[derive(Debug, Clone)]
pub struct DropTableStatement {
    table_name: String,
    if_exists: bool,
}

impl DropTableStatement {
    /// Creates a new `DropTableStatement`.
    pub fn new<N>(table_name: N) -> Self
    where
        N: Into<String>,
    {
        Self {
            table_name: table_name.into(),
            if_exists: false,
        }
    }

    /// Adds the `IF EXISTS` SQL to the `DROP TABLE` SQL statement.
    pub fn if_exists(self) -> Self {
        Self {
            if_exists: true,
            ..self
        }
    }
}

impl<DB: Backend> QueryFragment<DB> for DropTableStatement {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("DROP TABLE ");
        if self.if_exists {
            out.push_sql("IF EXISTS ");
        }
        out.push_identifier(&self.table_name)?;
        Ok(())
    }
}

impl<Conn> RunQueryDsl<Conn> for DropTableStatement {}

impl QueryId for DropTableStatement {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

/// Represents an incomplete SQL `CREATE TABLE` statement.
#[derive(Debug, Clone)]
pub struct IncompleteCreateTableStatement {
    table_name: String,
    if_not_exists: bool,
}

impl IncompleteCreateTableStatement {
    /// Creates a new `IncompleteCreateTableStatement`.
    pub fn new<N>(table_name: N) -> Self
    where
        N: Into<String>,
    {
        Self {
            table_name: table_name.into(),
            if_not_exists: false,
        }
    }

    pub fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }

    /// Adds the column definition to the `CREATE TABLE` SQL statement.
    pub fn columns(self, columns: CreateTableColumnDefinition) -> CreateTableStatement {
        CreateTableStatement {
            table_name: self.table_name,
            if_not_exists: self.if_not_exists,
            columns,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    column_name: String,
    column_type: String, // TO-DO: type safe
    null: bool,
}

impl ColumnDefinition {
    pub fn new<N, T>(column_name: N, column_type: T) -> Self
    where
        N: Into<String>,
        T: Into<String>,
    {
        Self {
            column_name: column_name.into(),
            column_type: column_type.into(),
            null: true,
        }
    }
}

impl<DB: Backend> QueryFragment<DB> for ColumnDefinition {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_identifier(&self.column_name)?;
        out.push_sql(&self.column_type); // TO-DO: prevent SQL injection here
        if !self.null {
            out.push_sql(" NOT");
        }
        out.push_sql(" NULL ");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TableColumnDefinition {
    columns: Vec<ColumnDefinition>,
}

impl<DB: Backend> QueryFragment<DB> for TableColumnDefinition {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        if self.columns.is_empty() {
            return Err(Error::QueryBuilderError(
                "There are no columns for the table definition. This query cannot be built".into(),
            ));
        }

        for (index, column) in self.columns.iter().enumerate() {
            column.walk_ast(out.reborrow())?;
            if index != self.columns.len() - 1 {
                out.push_sql(",");
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreateTableColumnDefinition {
    columns: TableColumnDefinition,
    primary_key: PrimaryKey,
}

impl<DB: Backend> QueryFragment<DB> for CreateTableColumnDefinition {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("(");
        self.columns.walk_ast(out.reborrow())?;
        self.primary_key.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PrimaryKey {
    keys: Vec<String>,
}

impl PrimaryKey {
    fn primary_key<K>(mut self, key: K) -> Self
    where
        K: Into<String>,
    {
        self.keys.push(key.into());
        self
    }
}

impl<DB: Backend> QueryFragment<DB> for PrimaryKey {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("PRIMARY KEY (");
        for key in &self.keys {
            out.push_identifier(&key)?;
        }
        out.push_sql(")");
        Ok(())
    }
}

/// Represents a SQL `CREATE TABLE` statement.
#[derive(Debug, Clone)]
pub struct CreateTableStatement {
    table_name: String,
    if_not_exists: bool,
    columns: CreateTableColumnDefinition,
}

impl CreateTableStatement {
    fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }
}

impl<DB: Backend> QueryFragment<DB> for CreateTableStatement {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("CREATE TABLE ");
        if self.if_not_exists {
            out.push_sql("IF NOT EXISTS ");
        }
        out.push_identifier(&self.table_name)?;
        self.columns.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Conn> RunQueryDsl<Conn> for CreateTableStatement {}

impl QueryId for CreateTableStatement {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

/// Generates the SQL `DROP TABLE` statement.
pub fn drop_table(table_name: &str) -> DropTableStatement {
    DropTableStatement::new(table_name)
}

/// Generates the SQL `CREATE TABLE` statement.
pub fn create_table(table_name: &str) -> IncompleteCreateTableStatement {
    IncompleteCreateTableStatement::new(table_name)
}
