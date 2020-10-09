use super::backend::Pg;
use crate::query_builder::QueryBuilder;
use crate::result::QueryResult;

mod distinct_on;
mod limit_offset;
pub(crate) mod on_constraint;
mod query_fragment_impls;
pub use self::distinct_on::DistinctOnClause;

/// The PostgreSQL query builder
#[allow(missing_debug_implementations)]
#[derive(Default)]
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
        itoa::fmt(&mut self.sql, self.bind_idx).expect("int formating does not fail");
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
    use crate::query_builder::{AstPass, QueryFragment};
    use crate::sql_types::*;

    let query = crate::sql_query("SELECT $1, $2, $3")
        .bind::<Integer, _>(42)
        .bind::<Integer, _>(3)
        .bind::<Integer, _>(342);

    let mut query_builder = PgQueryBuilder::default();

    {
        let ast_pass = AstPass::<crate::pg::Pg>::to_sql(&mut query_builder);

        query.walk_ast(ast_pass).unwrap();
    }

    assert_eq!(query_builder.bind_idx, 3);
    assert_eq!(query_builder.sql, "SELECT $1, $2, $3");
}
