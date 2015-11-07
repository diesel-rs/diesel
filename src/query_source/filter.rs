use expression::SelectableExpression;
use types;
use {QuerySource};
use query_builder::{QueryBuilder, BuildQueryResult};

#[derive(Clone, Copy)]
pub struct FilteredQuerySource<T, P> {
    source: T,
    predicate: P,
}

impl<T, P> FilteredQuerySource<T, P> {
    pub fn new(source: T, predicate: P) -> Self {
        FilteredQuerySource {
            source: source,
            predicate: predicate,
        }
    }
}

impl<T, P> QuerySource for FilteredQuerySource<T, P> where
    T: QuerySource,
    P: SelectableExpression<T, types::Bool>,
{
    type SqlType = T::SqlType;

    fn select_clause(&self) -> String {
        self.source.select_clause()
    }

    fn from_clause(&self) -> String {
        self.source.from_clause()
    }

    fn where_clause(&self) -> Option<(String, Vec<Option<Vec<u8>>>)> {
        Some((self.predicate.to_sql(), self.predicate.binds()))
    }

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("SELECT ");
        out.push_sql(&self.select_clause());
        out.push_sql(" FROM ");
        out.push_sql(&self.from_clause());
        if let Some((sql, mut binds)) = self.where_clause() {
            out.push_sql(" WHERE ");
            out.push_sql(&sql);
            out.push_binds(&mut binds);
        }
        Ok(())
    }
}
