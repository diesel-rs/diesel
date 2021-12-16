use diesel::backend::Backend;
use diesel::query_builder::*;
use diesel::result::QueryResult;
use diesel::RunQueryDsl;

#[derive(Debug, Clone)]
pub struct DropDatabaseStatement {
    db_name: String,
    if_exists: bool,
}

impl DropDatabaseStatement {
    pub fn new(db_name: &str) -> Self {
        DropDatabaseStatement {
            db_name: db_name.to_owned(),
            if_exists: false,
        }
    }

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

#[derive(Debug, Clone)]
pub struct CreateDatabaseStatement {
    db_name: String,
}

impl CreateDatabaseStatement {
    pub fn new(db_name: &str) -> Self {
        CreateDatabaseStatement {
            db_name: db_name.to_owned(),
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

pub fn drop_database(db_name: &str) -> DropDatabaseStatement {
    DropDatabaseStatement::new(db_name)
}

pub fn create_database(db_name: &str) -> CreateDatabaseStatement {
    CreateDatabaseStatement::new(db_name)
}
