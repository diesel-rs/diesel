use backend::Backend;
use insertable::*;
use pg::Pg;
use query_builder::*;
use query_builder::insert_statement::*;
use query_source::Table;
use result::QueryResult;

#[derive(Debug, Clone, Copy)]
pub struct OnConflictDoNothing<T>(T);

impl<T> OnConflictDoNothing<T> {
    pub fn new(records: T) -> Self {
        OnConflictDoNothing(records)
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

impl<'a, T, Tab> Insertable<Tab, Pg> for &'a OnConflictDoNothing<T> where
    Tab: Table,
    T: Insertable<Tab, Pg> + Copy,
    T: UndecoratedInsertRecord<Tab>,
{
    type Values = OnConflictDoNothingValues<T::Values>;

    fn values(self) -> Self::Values {
        OnConflictDoNothingValues(self.0.values())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct OnConflictDoNothingValues<T>(pub T);

impl<T> InsertValues<Pg> for OnConflictDoNothingValues<T> where
    T: InsertValues<Pg>,
{
    fn column_names(&self, out: &mut <Pg as Backend>::QueryBuilder) -> BuildQueryResult {
        self.0.column_names(out)
    }

    fn values_clause(&self, out: &mut <Pg as Backend>::QueryBuilder) -> BuildQueryResult {
        try!(self.0.values_clause(out));
        out.push_sql(" ON CONFLICT DO NOTHING");
        Ok(())
    }

    fn values_bind_params(&self, out: &mut <Pg as Backend>::BindCollector) -> QueryResult<()> {
        self.0.values_bind_params(out)
    }
}
