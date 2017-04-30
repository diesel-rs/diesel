use query_builder::AsQuery;
use query_source::{joins, QuerySource, JoinTo};

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors
pub trait InternalJoinDsl<Rhs, Kind, On> {
    type Output: AsQuery;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output;
}

impl<T, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for T where
    T: QuerySource + AsQuery,
    T::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <T::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors and grab
/// the known on clause from the associations API
pub trait JoinWithImplicitOnClause<Rhs, Kind> {
    type Output: AsQuery;

    fn join_with_implicit_on_clause(self, rhs: Rhs, kind: Kind) -> Self::Output;
}

impl<Lhs, Rhs, Kind> JoinWithImplicitOnClause<Rhs, Kind> for Lhs where
    Lhs: JoinTo<Rhs>,
    Lhs: InternalJoinDsl<Rhs, Kind, <Lhs as JoinTo<Rhs>>::JoinOnClause>,
{
    type Output = <Lhs as InternalJoinDsl<Rhs, Kind, Lhs::JoinOnClause>>::Output;

    fn join_with_implicit_on_clause(self, rhs: Rhs, kind: Kind) -> Self::Output {
        self.join(rhs, kind, Lhs::join_on_clause())
    }
}

pub trait JoinDsl: Sized {
    fn inner_join<Rhs>(self, rhs: Rhs) -> Self::Output where
        Self: JoinWithImplicitOnClause<Rhs, joins::Inner>,
    {
        self.join_with_implicit_on_clause(rhs, joins::Inner)
    }

    fn left_outer_join<Rhs>(self, rhs: Rhs) -> Self::Output where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.join_with_implicit_on_clause(rhs, joins::LeftOuter)
    }
}

impl<T: AsQuery> JoinDsl for T {
}
