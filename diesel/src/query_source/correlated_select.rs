use core::marker::PhantomData;

use crate::backend::Backend;
use crate::expression::array_comparison::AsInExpression;
use crate::expression::subselect::{Subselect, ValidSubselect, ValidSubselectGrouping};
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::{AstPass, QueryFragment, QueryId, SelectQuery};
use crate::query_dsl::lateral_join_dsl::private::LateralSource;
use crate::query_dsl::methods::*;
use crate::query_dsl::{InternalJoinDsl, QueryDsl};
use crate::query_source::joins::{Inner, Join};
use crate::query_source::{FunctionCall, QuerySource};
use crate::result::QueryResult;
use crate::sql_types::SqlType;

/// A function subselect whose arguments are checked against its enclosing query.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct CorrelatedSelect<Query, Call> {
    query: Query,
    call: PhantomData<Call>,
}

impl<Query, Call> CorrelatedSelect<Query, Call> {
    pub(crate) fn new(query: Query) -> Self {
        Self {
            query,
            call: PhantomData,
        }
    }
}

impl<Query, Call> QueryDsl for CorrelatedSelect<Query, Call> {}

impl<Query: SelectQuery, Call> SelectQuery for CorrelatedSelect<Query, Call> {
    type SqlType = Query::SqlType;
}

impl<Query, Call, DB> QueryFragment<DB> for CorrelatedSelect<Query, Call>
where
    DB: Backend,
    Query: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.query.walk_ast(out)
    }
}

impl<Query, R, F, QS> ValidSubselect<QS> for CorrelatedSelect<Query, FunctionCall<R, F>>
where
    Query: ValidSubselect<QS> + LateralSource,
    Query::Source: QuerySource,
    QS: QuerySource,
    Join<Query::Source, QS, Inner>: QuerySource,
    F: AppearsOnTable<QS>,
{
}

impl<Query, R, F, GB> ValidSubselectGrouping<GB> for CorrelatedSelect<Query, FunctionCall<R, F>>
where
    Query: ValidSubselectGrouping<GB>,
    F: crate::expression::ValidGrouping<GB>,
    Query::IsAggregate: crate::expression::MixedAggregates<F::IsAggregate>,
{
    type IsAggregate =
        <Query::IsAggregate as crate::expression::MixedAggregates<F::IsAggregate>>::Output;
}

impl<Query, Call, ST> AsInExpression<ST> for CorrelatedSelect<Query, Call>
where
    ST: SqlType,
    Self: SelectQuery<SqlType = ST>,
    Subselect<Self, ST>: Expression<SqlType = ST>,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

impl<Query, Call, Predicate> FilterDsl<Predicate> for CorrelatedSelect<Query, Call>
where
    Query: FilterDsl<Predicate>,
    Predicate: Expression,
{
    type Output = CorrelatedSelect<<Query as FilterDsl<Predicate>>::Output, Call>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        CorrelatedSelect::new(self.query.filter(predicate))
    }
}

impl<Query, Call, Predicate> OrFilterDsl<Predicate> for CorrelatedSelect<Query, Call>
where
    Query: OrFilterDsl<Predicate>,
    Predicate: Expression,
{
    type Output = CorrelatedSelect<<Query as OrFilterDsl<Predicate>>::Output, Call>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        CorrelatedSelect::new(self.query.or_filter(predicate))
    }
}

impl<Query, Call, Selection> SelectDsl<Selection> for CorrelatedSelect<Query, Call>
where
    Query: SelectDsl<Selection>,
    Selection: Expression,
{
    type Output = CorrelatedSelect<<Query as SelectDsl<Selection>>::Output, Call>;

    fn select(self, selection: Selection) -> Self::Output {
        CorrelatedSelect::new(self.query.select(selection))
    }
}

impl<Query, Call> DistinctDsl for CorrelatedSelect<Query, Call>
where
    Query: DistinctDsl,
{
    type Output = CorrelatedSelect<<Query as DistinctDsl>::Output, Call>;

    fn distinct(self) -> Self::Output {
        CorrelatedSelect::new(self.query.distinct())
    }
}

#[cfg(feature = "postgres_backend")]
impl<Query, Call, Selection> DistinctOnDsl<Selection> for CorrelatedSelect<Query, Call>
where
    Query: DistinctOnDsl<Selection>,
    Selection: Expression,
{
    type Output = CorrelatedSelect<<Query as DistinctOnDsl<Selection>>::Output, Call>;

    fn distinct_on(self, selection: Selection) -> Self::Output {
        CorrelatedSelect::new(self.query.distinct_on(selection))
    }
}

impl<Query, Call, Grouping> GroupByDsl<Grouping> for CorrelatedSelect<Query, Call>
where
    Query: GroupByDsl<Grouping>,
    Grouping: Expression,
{
    type Output = CorrelatedSelect<<Query as GroupByDsl<Grouping>>::Output, Call>;

    fn group_by(self, expression: Grouping) -> Self::Output {
        CorrelatedSelect::new(self.query.group_by(expression))
    }
}

impl<Query, Call, Predicate> HavingDsl<Predicate> for CorrelatedSelect<Query, Call>
where
    Query: HavingDsl<Predicate>,
    Predicate: Expression,
{
    type Output = CorrelatedSelect<<Query as HavingDsl<Predicate>>::Output, Call>;

    fn having(self, predicate: Predicate) -> Self::Output {
        CorrelatedSelect::new(self.query.having(predicate))
    }
}

impl<Query, Call, Ordering> OrderDsl<Ordering> for CorrelatedSelect<Query, Call>
where
    Query: OrderDsl<Ordering>,
    Ordering: Expression,
{
    type Output = CorrelatedSelect<<Query as OrderDsl<Ordering>>::Output, Call>;

    fn order(self, expression: Ordering) -> Self::Output {
        CorrelatedSelect::new(self.query.order(expression))
    }
}

impl<Query, Call, Ordering> ThenOrderDsl<Ordering> for CorrelatedSelect<Query, Call>
where
    Query: ThenOrderDsl<Ordering>,
    Ordering: Expression,
{
    type Output = CorrelatedSelect<<Query as ThenOrderDsl<Ordering>>::Output, Call>;

    fn then_order_by(self, expression: Ordering) -> Self::Output {
        CorrelatedSelect::new(self.query.then_order_by(expression))
    }
}

impl<Query, Call> LimitDsl for CorrelatedSelect<Query, Call>
where
    Query: LimitDsl,
{
    type Output = CorrelatedSelect<<Query as LimitDsl>::Output, Call>;

    fn limit(self, limit: i64) -> Self::Output {
        CorrelatedSelect::new(self.query.limit(limit))
    }
}

impl<Query, Call> OffsetDsl for CorrelatedSelect<Query, Call>
where
    Query: OffsetDsl,
{
    type Output = CorrelatedSelect<<Query as OffsetDsl>::Output, Call>;

    fn offset(self, offset: i64) -> Self::Output {
        CorrelatedSelect::new(self.query.offset(offset))
    }
}

impl<Query, Call> SelectNullableDsl for CorrelatedSelect<Query, Call>
where
    Query: SelectNullableDsl,
{
    type Output = CorrelatedSelect<<Query as SelectNullableDsl>::Output, Call>;

    fn nullable(self) -> Self::Output {
        CorrelatedSelect::new(self.query.nullable())
    }
}

impl<'a, Query, Call, DB> BoxedDsl<'a, DB> for CorrelatedSelect<Query, Call>
where
    Query: BoxedDsl<'a, DB>,
{
    type Output = CorrelatedSelect<<Query as BoxedDsl<'a, DB>>::Output, Call>;

    fn internal_into_boxed(self) -> Self::Output {
        CorrelatedSelect::new(self.query.internal_into_boxed())
    }
}

impl<'a, Query, Call, DB> BoxedCloneDsl<'a, DB> for CorrelatedSelect<Query, Call>
where
    Query: BoxedCloneDsl<'a, DB>,
{
    type Output = CorrelatedSelect<<Query as BoxedCloneDsl<'a, DB>>::Output, Call>;

    fn internal_into_boxed_clone(self) -> Self::Output {
        CorrelatedSelect::new(self.query.internal_into_boxed_clone())
    }
}

impl<Query, Call, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for CorrelatedSelect<Query, Call>
where
    Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = CorrelatedSelect<<Query as InternalJoinDsl<Rhs, Kind, On>>::Output, Call>;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        CorrelatedSelect::new(self.query.join(rhs, kind, on))
    }
}

impl<Query, Call> LateralSource for CorrelatedSelect<Query, Call>
where
    Query: LateralSource,
{
    type Source = Query::Source;
    type Query = CorrelatedSelect<Query::Query, Call>;

    fn into_query(self) -> Self::Query {
        CorrelatedSelect::new(self.query.into_query())
    }
}

impl<Query, Call, Lock> LockingDsl<Lock> for CorrelatedSelect<Query, Call>
where
    Query: LockingDsl<Lock>,
{
    type Output = CorrelatedSelect<<Query as LockingDsl<Lock>>::Output, Call>;

    fn with_lock(self, lock: Lock) -> Self::Output {
        CorrelatedSelect::new(self.query.with_lock(lock))
    }
}

impl<Query, Call, Modifier> ModifyLockDsl<Modifier> for CorrelatedSelect<Query, Call>
where
    Query: ModifyLockDsl<Modifier>,
{
    type Output = CorrelatedSelect<<Query as ModifyLockDsl<Modifier>>::Output, Call>;

    fn modify_lock(self, modifier: Modifier) -> Self::Output {
        CorrelatedSelect::new(self.query.modify_lock(modifier))
    }
}

impl<Query, Call, Rhs> crate::query_source::JoinTo<Rhs> for CorrelatedSelect<Query, Call>
where
    Query: crate::query_source::JoinTo<Rhs>,
{
    type FromClause = Query::FromClause;
    type OnClause = Query::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        Query::join_target(rhs)
    }
}
