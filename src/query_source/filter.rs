use expression::SelectableExpression;
use types;
use {QuerySource};

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
}
