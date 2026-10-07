use crate::query_builder::update_statement::batch_update::{
    BATCH_UPDATE_ALIAS, BatchAssignHelper, BatchKeyHelper, BatchUpdate, BatchValueHelper,
};
use crate::query_builder::update_statement::changeset::{Assign, ColumnWrapperForUpdate};
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::EmptyChangeset;
use crate::result::Error::QueryBuilderError;
use crate::{QueryResult, Table};

impl<C, T> BatchAssignHelper<crate::pg::Pg> for Assign<ColumnWrapperForUpdate<C>, T>
where
    ColumnWrapperForUpdate<C>: QueryFragment<crate::pg::Pg>,
{
    fn batch_assign_identifier<'b>(
        &'b self,
        mut out: AstPass<'_, 'b, crate::pg::Pg>,
    ) -> QueryResult<()> {
        self.target.walk_ast(out.reborrow())
    }
}

impl<Tab, I, C> QueryFragment<crate::pg::Pg, crate::pg::backend::PostgresLikeBatchUpdateSupport>
    for BatchUpdate<I, C, Tab::PrimaryKey, Tab>
where
    C: BatchValueHelper<crate::pg::Pg>,
    I: BatchKeyHelper<Tab::PrimaryKey, crate::pg::Pg>,
    Tab: Table,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::pg::Pg>) -> QueryResult<()> {
        // Always unsafe to cache since this does not have a static query id.
        out.unsafe_to_cache_prepared();

        let first = self
            .values
            .first()
            .ok_or_else(|| QueryBuilderError(Box::new(EmptyChangeset)))?;

        out.push_sql(" SET ");

        // --- Create this statement with the following steps:
        //
        // UPDATE my_table AS tab
        // SET
        //      column_a = tmp.column_a,
        //      column_c = tmp.column_c
        // FROM ( VALUES
        //      ('aa', 1, 11),
        //      ('bb', 2, 22)
        // ) AS tmp(column_a, column_b, column_c)
        // WHERE
        //      tab.column_b = tmp.column_b;

        // --- Assign target columns from temporary columns
        //
        //      column_a = tmp.column_a,
        //      column_c = tmp.column_c
        BatchValueHelper::assign(&first.1, out.reborrow())?;

        // --- List of values
        //
        // FROM ( VALUES
        //      ('aa', 1, 11),
        //      ('bb', 2, 22)
        // )
        out.push_sql(" FROM ( VALUES ");
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
        out.push_sql(" )");

        // --- Set alias and its columns
        //
        //      AS tmp(column_a, column_b, column_c)
        out.push_sql(" AS ");
        out.push_identifier(BATCH_UPDATE_ALIAS)?;
        out.push_sql("(");
        BatchKeyHelper::column_name(&first.0, out.reborrow())?;
        out.push_sql(", ");
        BatchValueHelper::column_name(&first.1, out.reborrow())?;
        out.push_sql(")");

        // --- Set equality condition for primary key(s)
        //
        // WHERE tab.column_b = tmp.column_b;
        out.push_sql(" WHERE ");
        <I as BatchKeyHelper<Tab::PrimaryKey, crate::pg::Pg>>::assign(&self.primary_key, out)?;
        Ok(())
    }
}
