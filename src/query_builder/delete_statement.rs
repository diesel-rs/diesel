use query_builder::*;

pub fn delete<T: UpdateTarget>(source: T) -> DeleteStatement<T> {
    DeleteStatement(source)
}

pub struct DeleteStatement<T>(T);

impl<T> QueryFragment for DeleteStatement<T> where
    T: UpdateTarget,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("DELETE FROM ");
        try!(self.0.from_clause(out));
        self.0.where_clause(out)
    }
}

