use crate::mysql_like::MysqlLikeBackend;
use crate::query_builder::update_statement::batch_update::{
    BATCH_UPDATE_ALIAS, BatchAssignHelper, BatchKeyHelper, BatchUpdate, BatchValueHelper,
};
use crate::query_builder::update_statement::changeset::{Assign, ColumnWrapperForUpdate};
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::EmptyChangeset;
use crate::result::Error::QueryBuilderError;
use crate::{QueryResult, Table};

impl<DB: MysqlLikeBackend, C, T> BatchAssignHelper<DB> for Assign<ColumnWrapperForUpdate<C>, T>
where
    C: QueryFragment<DB>,
{
    fn batch_assign_identifier<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.target.0.walk_ast(out.reborrow())
    }
}

impl<DB: MysqlLikeBackend, I, C, Tab>
    QueryFragment<DB, crate::mysql_like::query_fragments::MySqlLikeBatchUpdateSupport>
    for BatchUpdate<I, C, Tab::PrimaryKey, Tab>
where
    I: BatchKeyHelper<Tab::PrimaryKey, DB>,
    C: BatchValueHelper<DB>,
    Tab: Table,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        // Always unsafe to cache since this does not have a static query id.
        out.unsafe_to_cache_prepared();

        // UPDATE my_table AS tab
        // JOIN ( WITH CTE (column_b, column_a, column_c) AS(
        //      SELECT 1, 10, 20
        //      UNION ALL SELECT 2, 5, 10
        //      UNION ALL SELECT 3, 15, 30) SELECT * FROM CTE
        // ) AS tmp
        // ON
        //      tab.column_b = tmp.column_b
        // SET
        //      tab.column_a = tmp.column_a,
        //      tab.column_c = tmp.column_c;

        let first = self
            .values
            .first()
            .ok_or_else(|| QueryBuilderError(Box::new(EmptyChangeset)))?;

        out.push_sql(" JOIN ( WITH  ");
        out.push_identifier("CTE")?;
        out.push_sql("(");
        BatchKeyHelper::column_name(&first.0, out.reborrow())?;
        out.push_sql(", ");
        BatchValueHelper::column_name(&first.1, out.reborrow())?;
        out.push_sql(") AS (");
        let mut values = self.values.iter();
        if let Some((key, value)) = values.next() {
            out.push_sql(" SELECT ");
            BatchKeyHelper::bind_value(key, out.reborrow())?;
            out.push_sql(", ");
            BatchValueHelper::bind_value(value, out.reborrow())?;
        }
        for (key, value) in values {
            out.push_sql(" UNION ALL SELECT ");
            BatchKeyHelper::bind_value(key, out.reborrow())?;
            out.push_sql(", ");
            BatchValueHelper::bind_value(value, out.reborrow())?;
        }
        out.push_sql(" ) SELECT * FROM ");
        out.push_identifier("CTE")?;
        out.push_sql(")");

        out.push_sql(" AS ");
        out.push_identifier(BATCH_UPDATE_ALIAS)?;

        out.push_sql(" ON ");

        <I as BatchKeyHelper<Tab::PrimaryKey, DB>>::assign(&self.primary_key, out.reborrow())?;
        out.push_sql(" SET ");
        BatchValueHelper::assign(&first.1, out.reborrow())?;

        Ok(())
    }
}
