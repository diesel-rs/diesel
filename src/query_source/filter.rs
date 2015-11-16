use expression::{Expression, NonAggregate};
use query_builder::*;
use query_dsl::{FilterDsl, FilterOutput};
use query_source::QuerySource;
use types::Bool;

pub struct FilteredQuerySource<Source, Predicate> {
    source: Source,
    predicate: Predicate,
}

impl<Source, Predicate> FilteredQuerySource<Source, Predicate> {
    pub fn new(source: Source, predicate: Predicate) -> Self {
        FilteredQuerySource {
            source: source,
            predicate: predicate,
        }
    }
}

impl<Source, Predicate> AsQuery for FilteredQuerySource<Source, Predicate> where
    Predicate: Expression<SqlType=Bool> + NonAggregate,
    Source: QuerySource + AsQuery,
    Source::Query: FilterDsl<Predicate>,
{
    type SqlType = <FilterOutput<Source::Query, Predicate> as AsQuery>::SqlType;
    type Query = <FilterOutput<Source::Query, Predicate> as AsQuery>::Query;

    fn as_query(self) -> Self::Query {
        self.source.as_query().filter(self.predicate)
            .as_query()
    }
}

impl<Source, Predicate> QuerySource for FilteredQuerySource<Source, Predicate> where
    Source: QuerySource,
{
    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        self.source.from_clause(out)
    }
}
