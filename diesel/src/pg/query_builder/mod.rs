use super::backend::Pg;
use crate::query_builder::QueryBuilder;
use crate::result::QueryResult;

pub(crate) mod copy;
mod distinct_on;
mod limit_offset;
pub(crate) mod on_constraint;
pub(crate) mod only;
mod query_fragment_impls;
pub(crate) mod tablesample;
pub use self::copy::{CopyFormat, CopyFromQuery, CopyHeader, CopyTarget, CopyToQuery};
pub use self::distinct_on::DistinctOnClause;
pub use self::distinct_on::OrderDecorator;

/// The PostgreSQL query builder
#[allow(missing_debug_implementations)]
#[derive(Default)]
#[cfg(feature = "postgres_backend")]
pub struct PgQueryBuilder {
    sql: String,
    bind_idx: u32,
}

impl PgQueryBuilder {
    /// Constructs a new query builder with an empty query
    pub fn new() -> Self {
        PgQueryBuilder::default()
    }
}

impl QueryBuilder<Pg> for PgQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        self.push_sql("\"");
        self.push_sql(&identifier.replace('"', "\"\""));
        self.push_sql("\"");
        Ok(())
    }

    fn push_bind_param(&mut self) {
        self.push_bind_param_value_only();
        self.sql += "$";
        let mut buffer = itoa::Buffer::new();
        self.sql += buffer.format(self.bind_idx);
    }

    fn push_bind_param_value_only(&mut self) {
        self.bind_idx += 1;
    }

    fn finish(self) -> String {
        self.sql
    }
}

#[test]
fn check_sql_query_increments_bind_count() {
    use crate::query_builder::{AstPass, AstPassToSqlOptions, QueryFragment};
    use crate::sql_types::*;

    let query = crate::sql_query("SELECT $1, $2, $3")
        .bind::<Integer, _>(42)
        .bind::<Integer, _>(3)
        .bind::<Integer, _>(342);

    let mut query_builder = PgQueryBuilder::default();

    {
        let mut options = AstPassToSqlOptions::default();
        let ast_pass = AstPass::<crate::pg::Pg>::to_sql(&mut query_builder, &mut options, &Pg);

        query.walk_ast(ast_pass).unwrap();
    }

    assert_eq!(query_builder.bind_idx, 3);
    assert_eq!(query_builder.sql, "SELECT $1, $2, $3");
}
