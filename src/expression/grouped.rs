use expression::Expression;
use query_builder::{QueryBuilder, BuildQueryResult};

pub struct Grouped<T>(pub T);

impl<T: Expression> Expression for Grouped<T> {
    type SqlType = T::SqlType;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("(");
        try!(self.0.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn to_insert_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("(");
        try!(self.0.to_insert_sql(out));
        out.push_sql(")");
        Ok(())
    }
}
