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

    fn select_clause<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        self.source.select_clause(out)
    }

    fn from_clause<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        self.source.from_clause(out)
    }

    fn where_clause<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql(" WHERE ");
        self.predicate.to_sql(out)
    }
}
