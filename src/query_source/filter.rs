use expression::{Expression, SelectableExpression, NonAggregate};
use expression::predicates::And;
use query_builder::*;
use query_dsl::{FilterDsl, FilterOutput};
use query_source::QuerySource;
use types::Bool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Predicate: SelectableExpression<Source, SqlType=Bool> + NonAggregate,
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

impl<Source, Predicate, NewPredicate> FilterDsl<NewPredicate>
    for FilteredQuerySource<Source, Predicate> where
        FilteredQuerySource<Source, And<Predicate, NewPredicate>>: AsQuery,
        Predicate: SelectableExpression<Source, SqlType=Bool>,
        NewPredicate: SelectableExpression<Source, SqlType=Bool> + NonAggregate,
{
    type Output = FilteredQuerySource<Source, And<Predicate, NewPredicate>>;

    fn filter(self, predicate: NewPredicate) -> Self::Output {
        FilteredQuerySource::new(self.source, self.predicate.and(predicate))
    }
}

impl<Source, Predicate> QuerySource for FilteredQuerySource<Source, Predicate> where
    Source: QuerySource,
{
    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        self.source.from_clause(out)
    }
}

impl<Source, Predicate> UpdateTarget for FilteredQuerySource<Source, Predicate> where
    Source: UpdateTarget,
    Predicate: SelectableExpression<Source, SqlType=Bool>,
{
    type Table = Source::Table;

    fn where_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql(" WHERE ");
        self.predicate.to_sql(out)
    }

    fn table(&self) -> &Self::Table {
        self.source.table()
    }
}
