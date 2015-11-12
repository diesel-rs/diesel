use expression::{Expression, NonAggregate};
use query_builder::{Query, AsQuery};
use query_source::QuerySource;
use types::Bool;

pub trait FilterDsl<Predicate: Expression<SqlType=Bool> + NonAggregate> {
    type Output: Query;

    fn filter(self, predicate: Predicate) -> Self::Output;
}

impl<T, Predicate> FilterDsl<Predicate> for T where
    Predicate: Expression<SqlType=Bool> + NonAggregate,
    T: QuerySource + AsQuery,
    T::Query: FilterDsl<Predicate>,
{
    type Output = <T::Query as FilterDsl<Predicate>>::Output;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}
