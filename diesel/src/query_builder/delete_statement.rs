use query_builder::*;

pub struct DeleteStatement<T>(T);

impl<T> DeleteStatement<T> {
    #[doc(hidden)]
    pub fn new(t: T) -> Self {
        DeleteStatement(t)
    }
}

impl<T> QueryFragment for DeleteStatement<T> where
    T: UpdateTarget,
    T::WhereClause: QueryFragment,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_context(Context::Delete);
        out.push_sql("DELETE FROM ");
        try!(self.0.from_clause(out));
        if let Some(clause) =  self.0.where_clause() {
            out.push_sql(" WHERE ");
            try!(clause.to_sql(out));
        }
        out.pop_context();
        Ok(())
    }
}

