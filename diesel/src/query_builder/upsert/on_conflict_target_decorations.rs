use crate::backend::{Backend, SupportsOnConflictClause};
use crate::expression::operators::And;
use crate::expression::Expression;
use crate::query_builder::where_clause::{NoWhereClause, WhereAnd, WhereClause};
use crate::query_builder::{AstPass, QueryFragment, QueryResult};
use crate::query_dsl::methods::FilterDsl;
use crate::sql_types::Bool;

pub struct UndecoratedConflictTarget<T>(pub T);

#[derive(Debug)]
pub struct FilteredConflictTarget<T, P> {
    target: T,
    where_clause: WhereClause<P>,
}

impl<DB, T, U> QueryFragment<DB> for FilteredConflictTarget<T, U>
where
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
    DB: Backend + SupportsOnConflictClause,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.target.walk_ast(out.reborrow())?;
        out.push_sql(" ");
        self.where_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T, Predicate> FilterDsl<Predicate> for UndecoratedConflictTarget<T>
where
    Predicate: Expression<SqlType = Bool>,
{
    type Output = FilteredConflictTarget<T, Predicate>;
    fn filter(self, predicate: Predicate) -> Self::Output {
        FilteredConflictTarget {
            target: self.0,
            where_clause: NoWhereClause.and(predicate),
        }
    }
}

impl<T, P1, P2> FilterDsl<P2> for FilteredConflictTarget<T, P1>
where
    P1: Expression<SqlType = Bool>,
    P2: Expression<SqlType = Bool>,
{
    type Output = FilteredConflictTarget<T, And<P1, P2>>;
    fn filter(self, predicate: P2) -> Self::Output {
        FilteredConflictTarget {
            target: self.target,
            where_clause: self.where_clause.and(predicate),
        }
    }
}
