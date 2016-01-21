use backend::Backend;
use query_builder::*;

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
        out.push_context(Context::Delete);
        out.push_sql("DELETE FROM ");
        try!(self.0.from_clause().to_sql(out));
        if let Some(clause) = self.0.where_clause() {
            out.push_sql(" WHERE ");
            try!(clause.to_sql(out));
        }
        out.pop_context();
        Ok(())
    }
}

