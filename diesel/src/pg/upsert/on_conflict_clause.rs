use backend::Backend;
use insertable::*;
use pg::Pg;
use query_builder::*;
use query_source::Table;
use result::QueryResult;
use super::on_conflict_actions::*;
use super::on_conflict_target::*;

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConflictValues<Values, Target, Action> {
    values: Values,
    target: Target,
    action: Action,
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

impl<Values, Target, Action> CanInsertInSingleQuery<Pg> for OnConflictValues<Values, Target, Action>
where
    Values: CanInsertInSingleQuery<Pg>,
{
    fn rows_to_insert(&self) -> usize {
        self.values.rows_to_insert()
    }
}

impl<Tab, Values, Target, Action> InsertValues<Tab, Pg> for OnConflictValues<Values, Target, Action>
where
    Tab: Table,
    Values: InsertValues<Tab, Pg>,
    Target: QueryFragment<Pg>,
    Action: QueryFragment<Pg>,
{
    fn column_names(&self, out: &mut <Pg as Backend>::QueryBuilder) -> QueryResult<()> {
        self.values.column_names(out)
    }

    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        if self.values.requires_parenthesis() {
            out.push_sql("(");
        }
        self.values.walk_ast(out.reborrow())?;
        if self.values.requires_parenthesis() {
            out.push_sql(")");
        }
        out.push_sql(" ON CONFLICT");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        Ok(())
    }

    fn is_noop(&self) -> bool {
        self.values.is_noop()
    }

    fn requires_parenthesis(&self) -> bool {
        false
    }
}
