use backend::Backend;
use query_builder::*;
use query_source::Table;
use result::QueryResult;

#[derive(Debug)]
pub struct DeleteStatement<T, U>(UpdateTarget<T, U>);

impl<T, U> DeleteStatement<T, U> {
    #[doc(hidden)]
    pub fn new(t: UpdateTarget<T, U>) -> Self {
        DeleteStatement(t)
    }
}

impl<T, U, DB> QueryFragment<DB> for DeleteStatement<T, U> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DELETE FROM ");
        try!(self.0.table.from_clause().to_sql(out));
        if let Some(ref clause) = self.0.where_clause {
            out.push_sql(" WHERE ");
            try!(clause.to_sql(out));
        }
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.0.table.from_clause().collect_binds(out));
        if let Some(ref clause) = self.0.where_clause {
            try!(clause.collect_binds(out));
        }
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.0.table.from_clause().is_safe_to_cache_prepared() &&
            self.0.where_clause.as_ref().map(|w| w.is_safe_to_cache_prepared()).unwrap_or(true)
    }
}

impl_query_id!(noop: DeleteStatement<T, U>);
