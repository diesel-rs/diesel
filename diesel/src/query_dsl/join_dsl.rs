use query_builder::AsQuery;
use query_source::{joins, QuerySource};

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors
pub trait InternalJoinDsl<Rhs, Kind> {
    type Output: AsQuery;

    fn join(self, rhs: Rhs, kind: Kind) -> Self::Output;
}

impl<T, Rhs, Kind> InternalJoinDsl<Rhs, Kind> for T where
    T: QuerySource + AsQuery,
    T::Query: InternalJoinDsl<Rhs, Kind>,
{
    type Output = <T::Query as InternalJoinDsl<Rhs, Kind>>::Output;

    fn join(self, rhs: Rhs, kind: Kind) -> Self::Output {
        self.as_query().join(rhs, kind)
    }
}

pub trait JoinDsl: Sized {
    fn inner_join<Rhs>(self, rhs: Rhs) -> Self::Output where
        Self: InternalJoinDsl<Rhs, joins::Inner>,
    {
        self.join(rhs, joins::Inner)
    }

    fn left_outer_join<Rhs>(self, rhs: Rhs) -> Self::Output where
        Self: InternalJoinDsl<Rhs, joins::LeftOuter>,
    {
        self.join(rhs, joins::LeftOuter)
    }
}

impl<T: AsQuery> JoinDsl for T {
}
