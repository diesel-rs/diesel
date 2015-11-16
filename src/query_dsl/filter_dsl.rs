use expression::{Expression, NonAggregate};
use query_builder::AsQuery;
use query_source::filter::FilteredQuerySource;
use types::Bool;

pub type FilterOutput<T, P> = <T as FilterDsl<P>>::Output;

pub trait FilterDsl<Predicate: Expression<SqlType=Bool> + NonAggregate> {
    type Output: AsQuery;

    fn filter(self, predicate: Predicate) -> Self::Output;
}

impl<T, Predicate> FilterDsl<Predicate> for T where
    Predicate: Expression<SqlType=Bool> + NonAggregate,
    FilteredQuerySource<T, Predicate>: AsQuery,
{
    type Output = FilteredQuerySource<Self, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        FilteredQuerySource::new(self, predicate)
    }
}
