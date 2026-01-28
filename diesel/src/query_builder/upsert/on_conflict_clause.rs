use super::on_conflict_actions::*;
use super::on_conflict_target::*;
use crate::backend::sql_dialect;
use crate::insertable::*;
use crate::query_builder::where_clause::{NoWhereClause, WhereClause};
use crate::query_builder::*;

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConflictValues<Values, Target, Action, WhereClause = NoWhereClause> {
    pub(crate) values: Values,
    pub(crate) target: Target,
    pub(crate) action: Action,
    /// Allow to apply filters on ON CONFLICT ... DO UPDATE ... WHERE ...
    pub(crate) where_clause: WhereClause,
}

impl<Values, Target, Action, WhereClause> QueryId
    for OnConflictValues<Values, Target, Action, WhereClause>
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Values, T> OnConflictValues<Values, NoConflictTarget, DoNothing<T>, NoWhereClause> {
    pub(crate) fn do_nothing(values: Values) -> Self {
        Self::new(values, NoConflictTarget, DoNothing::new(), NoWhereClause)
    }
}

impl<Values, Target, Action, WhereClause> OnConflictValues<Values, Target, Action, WhereClause> {
    pub(crate) fn new(
        values: Values,
        target: Target,
        action: Action,
        where_clause: WhereClause,
    ) -> Self {
        OnConflictValues {
            values,
            target,
            action,
            where_clause,
        }
    }

    pub(crate) fn replace_where<Where, F>(
        self,
        f: F,
    ) -> OnConflictValues<Values, Target, Action, Where>
    where
        F: FnOnce(WhereClause) -> Where,
    {
        OnConflictValues::new(self.values, self.target, self.action, f(self.where_clause))
    }
}

impl<DB, Values, Target, Action, WhereClause> CanInsertInSingleQuery<DB>
    for OnConflictValues<Values, Target, Action, WhereClause>
where
    DB: Backend,
    DB::OnConflictClause: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
    Values: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        self.values.rows_to_insert()
    }
}

impl<DB, Values, Target, Action> QueryFragment<DB>
    for OnConflictValues<Values, Target, Action, NoWhereClause>
where
    DB: Backend,
    DB::OnConflictClause: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB, Values, Target, Action, SD> QueryFragment<DB, SD>
    for OnConflictValues<Values, Target, Action, NoWhereClause>
where
    DB: Backend<OnConflictClause = SD>,
    SD: sql_dialect::on_conflict_clause::PgLikeOnConflictClause,
    Values: QueryFragment<DB>,
    Target: QueryFragment<DB>,
    Action: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(" ON CONFLICT");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<DB, Values, Target, Action, Expr> QueryFragment<DB>
    for OnConflictValues<Values, Target, Action, WhereClause<Expr>>
where
    DB: Backend,
    DB::OnConflictClause: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
    DB::OnConflictClause: sql_dialect::on_conflict_clause::SupportsOnConflictClauseWhere,
    Values: QueryFragment<DB>,
    Target: QueryFragment<DB>,
    Action: QueryFragment<DB>,
    WhereClause<Expr>: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(" ON CONFLICT");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}
