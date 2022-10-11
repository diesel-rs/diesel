use crate::backend::Backend;
use crate::expression::Expression;
use crate::query_builder::upsert::on_conflict_target::{ConflictTarget, NoConflictTarget};
use crate::query_builder::where_clause::{NoWhereClause, WhereAnd, WhereClause};
use crate::query_builder::{AstPass, QueryFragment, QueryResult};
use crate::sql_types::BoolOrNullableBool;

pub trait UndecoratedConflictTarget {}

impl UndecoratedConflictTarget for NoConflictTarget {}
impl<T> UndecoratedConflictTarget for ConflictTarget<T> {}

/// Interface to add information to conflict targets.
/// Designed to be open for further additions to conflict targets like constraints
pub trait DecoratableTarget<P> {
    /// Output type of filter_target operation
    type FilterOutput;
    /// equivalent to filter of FilterDsl but aimed at conflict targets
    fn filter_target(self, predicate: P) -> Self::FilterOutput;
}

#[derive(Debug)]
pub struct DecoratedConflictTarget<T, U> {
    pub(crate) target: T,
    pub(crate) where_clause: U,
}

impl<T, P> DecoratableTarget<P> for T
where
    P: Expression,
    P::SqlType: BoolOrNullableBool,
    T: UndecoratedConflictTarget,
{
    type FilterOutput = DecoratedConflictTarget<T, WhereClause<P>>;

    fn filter_target(self, predicate: P) -> Self::FilterOutput {
        DecoratedConflictTarget {
            target: self,
            where_clause: NoWhereClause.and(predicate),
        }
    }
}

impl<T, U, P> DecoratableTarget<P> for DecoratedConflictTarget<T, U>
where
    P: Expression,
    P::SqlType: BoolOrNullableBool,
    U: WhereAnd<P>,
{
    type FilterOutput = DecoratedConflictTarget<T, <U as WhereAnd<P>>::Output>;

    fn filter_target(self, predicate: P) -> Self::FilterOutput {
        DecoratedConflictTarget {
            target: self.target,
            where_clause: self.where_clause.and(predicate),
        }
    }
}

impl<DB, T, U> QueryFragment<DB> for DecoratedConflictTarget<T, U>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}
