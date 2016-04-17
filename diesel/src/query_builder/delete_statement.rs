use backend::Backend;
use query_builder::*;
use result::QueryResult;

pub struct DeleteStatement<T>(T);

impl<T> DeleteStatement<T> {
    #[doc(hidden)]
    pub fn new(t: T) -> Self {
        DeleteStatement(t)
    }
}

impl<T, DB> QueryFragment<DB> for DeleteStatement<T> where
    DB: Backend,
    T: UpdateTarget,
    T::WhereClause: QueryFragment<DB>,
    T::FromClause: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DELETE FROM ");
        try!(self.0.from_clause().to_sql(out));
        if let Some(clause) = self.0.where_clause() {
            out.push_sql(" WHERE ");
            try!(clause.to_sql(out));
        }
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.0.from_clause().collect_binds(out));
        if let Some(clause) = self.0.where_clause() {
            try!(clause.collect_binds(out));
        }
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.0.from_clause().is_safe_to_cache_prepared() &&
            self.0.where_clause().map(|w| w.is_safe_to_cache_prepared()).unwrap_or(true)
    }
}

