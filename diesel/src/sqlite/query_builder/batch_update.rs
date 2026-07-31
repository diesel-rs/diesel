use crate::query_builder::update_statement::batch_update::{
    BATCH_UPDATE_ALIAS, BatchAssignHelper, BatchKeyHelper, BatchUpdate, BatchValueHelper,
};
use crate::query_builder::update_statement::changeset::{Assign, ColumnWrapperForUpdate};
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::EmptyChangeset;
use crate::result::Error::QueryBuilderError;
use crate::{QueryResult, Table};

impl<C, T> BatchAssignHelper<crate::sqlite::Sqlite> for Assign<ColumnWrapperForUpdate<C>, T>
where
    ColumnWrapperForUpdate<C>: QueryFragment<crate::sqlite::Sqlite>,
{
    fn batch_assign_identifier<'b>(
        &'b self,
        mut out: AstPass<'_, 'b, crate::sqlite::Sqlite>,
    ) -> QueryResult<()> {
        self.target.walk_ast(out.reborrow())
    }
}

impl<I, C, Tab> QueryFragment<crate::sqlite::Sqlite, crate::sqlite::backend::SqliteBatchUpdate>
    for BatchUpdate<I, C, Tab::PrimaryKey, Tab>
where
    I: BatchKeyHelper<Tab::PrimaryKey, crate::sqlite::Sqlite>,
    C: BatchValueHelper<crate::sqlite::Sqlite>,
    Tab: Table,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        // Always unsafe to cache since this does not have a static query id.
        out.unsafe_to_cache_prepared();

        out.push_sql(" SET ");
        // for sqlite:
        // update test set name = source.name from (with cte(id, name) as (values (1, 'boom'), (2, 'baz')) select * from cte)
        // as source where source.id = test.id;

        let first = self
            .values
            .first()
            .ok_or_else(|| QueryBuilderError(Box::new(EmptyChangeset)))?;

        BatchValueHelper::assign(&first.1, out.reborrow())?;

        out.push_sql(" FROM( WITH ");
        out.push_identifier("CTE")?;
        out.push_sql("(");
        BatchKeyHelper::column_name(&first.0, out.reborrow())?;
        out.push_sql(", ");
        BatchValueHelper::column_name(&first.1, out.reborrow())?;
        out.push_sql(") AS ( VALUES");
        let mut values = self.values.iter();
        if let Some((key, value)) = values.next() {
            out.push_sql("(");
            BatchKeyHelper::bind_value(key, out.reborrow())?;
            out.push_sql(", ");
            BatchValueHelper::bind_value(value, out.reborrow())?;
            out.push_sql(")");
        }
        for (key, value) in values {
            out.push_sql(", (");
            BatchKeyHelper::bind_value(key, out.reborrow())?;
            out.push_sql(", ");
            BatchValueHelper::bind_value(value, out.reborrow())?;
            out.push_sql(")");
        }
        out.push_sql(" ) SELECT * FROM ");
        out.push_identifier("CTE")?;
        out.push_sql(")");

        out.push_sql(" AS ");
        out.push_identifier(BATCH_UPDATE_ALIAS)?;
        out.push_sql(" WHERE ");
        <I as BatchKeyHelper<Tab::PrimaryKey, crate::sqlite::Sqlite>>::assign(
            &self.primary_key,
            out,
        )?;

        Ok(())
    }
}
