use super::on_conflict_actions::*;
use super::on_conflict_target::*;
use crate::backend::{Backend, SupportsOnConflictClause};
use crate::insertable::*;
use crate::query_builder::*;
use crate::result::QueryResult;

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConflictValues<Values, Target, Action> {
    values: Values,
    target: Target,
    action: Action,
}

impl<Values, Target, Action> QueryId for OnConflictValues<Values, Target, Action> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Values> OnConflictValues<Values, NoConflictTarget, DoNothing> {
    pub(crate) fn do_nothing(values: Values) -> Self {
        Self::new(values, NoConflictTarget, DoNothing)
    }
}

impl<Values, Target, Action> OnConflictValues<Values, Target, Action> {
    pub(crate) fn new(values: Values, target: Target, action: Action) -> Self {
        OnConflictValues {
            values,
            target,
            action,
        }
    }
}

impl<DB, Values, Target, Action> CanInsertInSingleQuery<DB>
    for OnConflictValues<Values, Target, Action>
where
    DB: Backend + SupportsOnConflictClause,
    Values: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        self.values.rows_to_insert()
    }
}

impl<DB, Values, Target, Action> QueryFragment<DB> for OnConflictValues<Values, Target, Action>
where
    DB: Backend + SupportsOnConflictClause,
    Values: QueryFragment<DB>,
    Target: QueryFragment<DB>,
    Action: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(" ON CONFLICT");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        Ok(())
    }
}
