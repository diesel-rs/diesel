use associations::HasTable;
use expression::{AppearsOnTable, NonAggregate};
use expression::expression_methods::*;
use expression::predicates::And;
use helper_types::Filter;
use query_builder::*;
use query_dsl::FilterDsl;
use query_source::QuerySource;
use types::Bool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use="Queries are only executed when calling `load`, `get_result` or similar."]
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
    Predicate: AppearsOnTable<Source, SqlType=Bool> + NonAggregate,
    Source: QuerySource + AsQuery,
    Source::Query: FilterDsl<Predicate>,
{
    type SqlType = <Filter<Source::Query, Predicate> as AsQuery>::SqlType;
    type Query = <Filter<Source::Query, Predicate> as AsQuery>::Query;

    fn as_query(self) -> Self::Query {
        self.source.as_query().filter(self.predicate)
            .as_query()
    }
}

impl<Source, Predicate, NewPredicate, ST> FilterDsl<NewPredicate>
    for FilteredQuerySource<Source, Predicate> where
        FilteredQuerySource<Source, Predicate>: AsQuery<SqlType=ST>,
        FilteredQuerySource<Source, And<Predicate, NewPredicate>>: AsQuery<SqlType=ST>,
        Predicate: AppearsOnTable<Source, SqlType=Bool>,
        NewPredicate: AppearsOnTable<Source, SqlType=Bool> + NonAggregate,
{
    type Output = FilteredQuerySource<Source, And<Predicate, NewPredicate>>;

    fn filter(self, predicate: NewPredicate) -> Self::Output {
        FilteredQuerySource::new(self.source, self.predicate.and(predicate))
    }
}

impl<Source, Predicate> QuerySource for FilteredQuerySource<Source, Predicate> where
    Source: QuerySource,
{
    type FromClause = Source::FromClause;

    fn from_clause(&self) -> Self::FromClause {
        self.source.from_clause()
    }
}

impl_query_id!(FilteredQuerySource<Source, Predicate>);

impl<Source, Predicate> HasTable for FilteredQuerySource<Source, Predicate> where
    Source: HasTable,
{
    type Table = Source::Table;

    fn table() -> Self::Table {
        Source::table()
    }
}

impl<Source, Predicate> IntoUpdateTarget for FilteredQuerySource<Source, Predicate> where
    FilteredQuerySource<Source, Predicate>: HasTable,
    Predicate: AppearsOnTable<Source, SqlType=Bool>,
{
    type WhereClause = Predicate;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
        UpdateTarget {
            table: Self::table(),
            where_clause: Some(self.predicate),
        }
    }
}
