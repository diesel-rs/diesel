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
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_context(Context::Delete);
        out.push_sql("DELETE FROM ");
        try!(self.0.from_clause(out));
        try!(self.0.where_clause(out));
        out.pop_context();
        Ok(())
    }
}

