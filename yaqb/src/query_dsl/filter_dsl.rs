use expression::predicates::Eq;
use expression::{Expression, NonAggregate};
use expression::helper_types::AsExpr;
use query_builder::AsQuery;
use query_source::filter::FilteredQuerySource;
use query_source::{Table, InnerJoinSource, LeftOuterJoinSource};
use types::Bool;

pub type FilterOutput<T, P> = <T as FilterDsl<P>>::Output;
pub type FindByOutput<T, Lhs, Rhs> = FilterOutput<T, Eq<Lhs, AsExpr<Rhs, Lhs>>>;

pub trait FilterDsl<Predicate: Expression<SqlType=Bool> + NonAggregate> {
    type Output: AsQuery;

    fn filter(self, predicate: Predicate) -> Self::Output;
}

pub trait NotFiltered {
}

impl<T, Predicate> FilterDsl<Predicate> for T where
    Predicate: Expression<SqlType=Bool> + NonAggregate,
    FilteredQuerySource<T, Predicate>: AsQuery,
    T: NotFiltered,
{
    type Output = FilteredQuerySource<Self, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        FilteredQuerySource::new(self, predicate)
    }
}

impl<T: Table> NotFiltered for T {}
impl<Left, Right> NotFiltered for InnerJoinSource<Left, Right> {}
impl<Left, Right> NotFiltered for LeftOuterJoinSource<Left, Right> {}
