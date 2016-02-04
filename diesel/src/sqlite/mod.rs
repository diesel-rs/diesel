mod backend;
mod connection;
mod query_builder;
mod types;

pub use self::backend::{Sqlite, SqliteType};
pub use self::connection::SqliteConnection;
pub use self::query_builder::SqliteQueryBuilder;

use ::expression::dsl::now;

impl ::query_builder::QueryFragment<Sqlite> for now {
    fn to_sql(&self, out: &mut SqliteQueryBuilder) -> ::query_builder::BuildQueryResult {
        use ::query_builder::QueryBuilder;
        out.push_sql("'now'");
        Ok(())
    }
}
