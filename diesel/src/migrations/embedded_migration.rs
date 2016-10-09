use super::{Migration, RunMigrationsError};
use ::connection::SimpleConnection;

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct EmbeddedMigration {
    pub version: &'static str,
    pub up_sql: &'static str,
}

impl Migration for EmbeddedMigration {
    fn version(&self) -> &str {
        self.version
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        conn.batch_execute(self.up_sql).map_err(Into::into)
    }

    fn revert(&self, _conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        unreachable!()
    }
}
