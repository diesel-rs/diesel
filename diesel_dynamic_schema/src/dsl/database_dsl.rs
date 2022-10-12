//! This module contains database creation / deletion statements.

use diesel::{backend::Backend, query_builder::*, result::QueryResult, RunQueryDsl};

/// Represents a SQL `DROP DATABASE` statement.
#[derive(Debug, Clone)]
pub struct DropDatabaseStatement {
    db_name: String,
    if_exists: bool,
}

impl DropDatabaseStatement {
    /// Creates a new `DropDatabaseStatement`.
    pub fn new<N>(db_name: N) -> Self
    where
        N: Into<String>,
    {
        DropDatabaseStatement {
            db_name: db_name.into(),
            if_exists: false,
        }
    }

    /// Adds the `IF EXISTS` SQL to the `DROP DATABASE` SQL statement.
    pub fn if_exists(self) -> Self {
        DropDatabaseStatement {
            if_exists: true,
            ..self
        }
    }
}

impl<DB: Backend> QueryFragment<DB> for DropDatabaseStatement {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("DROP DATABASE ");
        if self.if_exists {
            out.push_sql("IF EXISTS ");
        }
        out.push_identifier(&self.db_name)?;
        Ok(())
    }
}

impl<Conn> RunQueryDsl<Conn> for DropDatabaseStatement {}

impl QueryId for DropDatabaseStatement {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

/// Represents a SQL `CREATE DATABASE` statement.
#[derive(Debug, Clone)]
pub struct CreateDatabaseStatement {
    db_name: String,
}

impl CreateDatabaseStatement {
    /// Creates a new `CreateDatabaseStatement`.
    pub fn new<N>(db_name: N) -> Self
    where
        N: Into<String>,
    {
        CreateDatabaseStatement {
            db_name: db_name.into(),
        }
    }
}

impl<DB: Backend> QueryFragment<DB> for CreateDatabaseStatement {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("CREATE DATABASE ");
        out.push_identifier(&self.db_name)?;
        Ok(())
    }
}

impl<Conn> RunQueryDsl<Conn> for CreateDatabaseStatement {}

impl QueryId for CreateDatabaseStatement {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

/// Creates the SQL `DROP DATABASE` statement.
pub fn drop_database(db_name: &str) -> DropDatabaseStatement {
    DropDatabaseStatement::new(db_name)
}

/// Creates the SQL `CREATE DATABASE` statement.
///
/// Note that since the database will still not exist when running this, the connection must not
/// have this database in its URL. A common practice is to use `postgres` as the database name for
/// PostgreSQL and `information_schema` as the database name for MySQL, since those databases are
/// always present in their respective DBMSs.
pub fn create_database(db_name: &str) -> CreateDatabaseStatement {
    CreateDatabaseStatement::new(db_name)
}
