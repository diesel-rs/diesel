use super::on_conflict_actions::*;
use super::on_conflict_target::*;
use crate::backend::sql_dialect;
use crate::backend::Backend;
use crate::insertable::*;
use crate::query_builder::where_clause::NoWhereClause;
use crate::query_builder::*;
use crate::result::QueryResult;

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConflictValues<Values, Target, Action, WhereClause = NoWhereClause> {
    values: Values,
    target: Target,
    action: Action,
    /// Allow to apply filters on ON CONFLICT ... DO UPDATE ... WHERE ...
    where_clause: WhereClause,
}

impl<Values, Target, Action, WhereClause> QueryId
    for OnConflictValues<Values, Target, Action, WhereClause>
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Values> OnConflictValues<Values, NoConflictTarget, DoNothing, NoWhereClause> {
    pub(crate) fn do_nothing(values: Values) -> Self {
        Self::new(values, NoConflictTarget, DoNothing, NoWhereClause)
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

impl<DB, Values, Target, Action, WhereClause> QueryFragment<DB>
    for OnConflictValues<Values, Target, Action, WhereClause>
where
    DB: Backend,
    DB::OnConflictClause: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
    Values: QueryFragment<DB>,
    Target: QueryFragment<DB>,
    Action: QueryFragment<DB>,
    WhereClause: QueryFragment<DB>,
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
