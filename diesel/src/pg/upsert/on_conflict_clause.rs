use backend::Backend;
use insertable::*;
use pg::Pg;
use query_builder::*;
use query_builder::insert_statement::*;
use query_source::Table;
use result::QueryResult;
use super::on_conflict_actions::*;
use super::on_conflict_target::*;

#[derive(Debug, Clone, Copy)]
pub struct OnConflictDoNothing<T>(T);

impl<T> OnConflictDoNothing<T> {
    pub fn new(records: T) -> Self {
        OnConflictDoNothing(records)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OnConflict<Records, Target, Action> {
    records: Records,
    target: Target,
    action: Action,
}

impl<Records, Target, Action> OnConflict<Records, Target, Action> {
    pub fn new(records: Records, target: Target, action: Action) -> Self {
        OnConflict {
            records: records,
            target: target,
            action: action,
        }
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a OnConflictDoNothing<T>
where
    T: Insertable<Tab> + Copy,
    T: UndecoratedInsertRecord<Tab>,
{
    type Values = OnConflictValues<T::Values, NoConflictTarget, DoNothing>;

    fn values(self) -> Self::Values {
        OnConflictValues {
            values: self.0.values(),
            target: NoConflictTarget,
            action: DoNothing,
        }
    }
}

impl<'a, Records, Target, Action, Tab> Insertable<Tab> for &'a OnConflict<Records, Target, Action>
where
    Records: Insertable<Tab> + Copy,
    Records: UndecoratedInsertRecord<Tab>,
    Target: OnConflictTarget<Tab> + Clone,
    Action: IntoConflictAction<Tab> + Copy,
{
    type Values = OnConflictValues<Records::Values, Target, Action::Action>;

    fn values(self) -> Self::Values {
        OnConflictValues {
            values: self.records.values(),
            target: self.target.clone(),
            action: self.action.into_conflict_action(),
        }
    }
}


#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConflictValues<Values, Target, Action> {
    values: Values,
    target: Target,
    action: Action,
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
