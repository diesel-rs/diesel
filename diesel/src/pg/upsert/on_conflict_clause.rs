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

impl<'a, T, Tab, Op> IntoInsertStatement<Tab, Op> for &'a OnConflictDoNothing<T> {
    type InsertStatement = InsertStatement<Tab, Self, Op>;

    fn into_insert_statement(self, target: Tab, operator: Op)
        -> Self::InsertStatement
    {
        InsertStatement::no_returning_clause(target, self, operator)
    }
}

impl<'a, Recods, Target, Action, Tab, Op> IntoInsertStatement<Tab, Op>
    for &'a OnConflict<Recods, Target, Action>
{
    type InsertStatement = InsertStatement<Tab, Self, Op>;

    fn into_insert_statement(self, target: Tab, operator: Op)
        -> Self::InsertStatement
    {
        InsertStatement::no_returning_clause(target, self, operator)
    }
}

impl<'a, T, Tab> Insertable<Tab, Pg> for &'a OnConflictDoNothing<T> where
    Tab: Table,
    T: Insertable<Tab, Pg> + Copy,
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

impl<'a, Records, Target, Action, Tab> Insertable<Tab, Pg>
    for &'a OnConflict<Records, Target, Action> where
        Tab: Table,
        Records: Insertable<Tab, Pg> + Copy,
        Records: UndecoratedInsertRecord<Tab>,
        Target: OnConflictTarget<Tab> + Copy,
        Action: IntoConflictAction<Tab> + Copy,
{
    type Values = OnConflictValues<Records::Values, Target, Action::Action>;

    fn values(self) -> Self::Values {
        OnConflictValues {
            values: self.records.values(),
            target: self.target,
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

impl<Values, Target, Action> InsertValues<Pg> for OnConflictValues<Values, Target, Action> where
    Values: InsertValues<Pg>,
    Target: QueryFragment<Pg>,
    Action: QueryFragment<Pg>,
{
    fn column_names(&self, out: &mut <Pg as Backend>::QueryBuilder) -> BuildQueryResult {
        self.values.column_names(out)
    }

    fn values_clause(&self, out: &mut <Pg as Backend>::QueryBuilder) -> BuildQueryResult {
        try!(self.values.values_clause(out));
        out.push_sql(" ON CONFLICT");
        try!(self.target.to_sql(out));
        try!(self.action.to_sql(out));
        Ok(())
    }

    fn values_bind_params(&self, out: &mut <Pg as Backend>::BindCollector) -> QueryResult<()> {
        try!(self.values.values_bind_params(out));
        try!(self.target.collect_binds(out));
        try!(self.action.collect_binds(out));
        Ok(())
    }
}
