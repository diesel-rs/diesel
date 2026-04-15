use crate::backend::{Backend, SqlDialect};
use crate::expression::TypedExpressionType;
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::EmptyChangeset;
use crate::result::Error::QueryBuilderError;
use crate::sql_types::{HasSqlType, SqlType};
use crate::{Expression, QueryResult, Table, query_builder::*};

impl<IT, CT, I, C, PK, Tab, DB>
    QueryFragment<DB, crate::mysql::backend::MySqlLikeBatchUpdateSupport>
    for BatchUpdate<I, C, PK, Tab>
where
    DB: Backend
        + SqlDialect<BatchUpdateSupport = crate::mysql::backend::MySqlLikeBatchUpdateSupport>,
    IT: SqlType + TypedExpressionType, // SqlType of I
    CT: SqlType + TypedExpressionType, // SqlType of C
    I: BatchValue<IT, Tab, DB>,
    C: Expression<SqlType = CT>
        + BatchColumn<Tab, DB>
        + BatchColumnAssign<Tab, DB>
        + BatchValue<CT, Tab, DB>,
    PK: Expression<SqlType = IT> + BatchColumnAssign<Tab, DB> + BatchColumn<Tab, DB>,
    Tab: Table<PrimaryKey = PK>,
    DB: Backend + HasSqlType<IT> + HasSqlType<CT>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        // Always unsafe to cache since this does not have a static query id.
        out.unsafe_to_cache_prepared();

        // TODO:
        // The following two alternatives would work if we could postpone
        // the 'SET' keyword. Already tested Option 2.
        //
        // Option 1:
        //
        // UPDATE my_table AS tab
        // JOIN ( SELECT
        //      1 as column_b, 10 as column_a, 20 as column_c
        //      UNION ALL SELECT 2, 5, 10
        //      UNION ALL SELECT 3, 15, 30
        // ) AS tmp
        // ON
        //      tab.column_b = tmp.column_b
        // SET
        //      tab.column_a = tmp.column_a,
        //      tab.column_c = tmp.column_c;

        // Option 2: mysql 8.x or above
        //
        // UPDATE my_table AS tab
        // JOIN ( VALUES
        //      ROW(1, 10, 20),
        //      ROW(2, 5, 10),
        //      ROW(3, 15, 30)
        // ) AS tmp(column_a, column_b, column_c)
        // ON
        //      tab.column_b = tmp.column_b
        // SET
        //      tab.column_a = tmp.column_a,
        //      tab.column_c = tmp.column_c;

        if self.values.is_empty() {
            return Err(QueryBuilderError(Box::new(EmptyChangeset)));
        }

        // Implementation for Option 2:

        let first = self.values.first().expect("missing batch update values");

        // --- List of values
        //
        // JOIN ( VALUES
        //      ROW(1, 10, 20),
        //      ROW(2, 5, 10),
        //      ROW(3, 15, 30)
        // )
        out.push_sql(" JOIN ( VALUES ");
        let mut values = self.values.iter();
        if let Some(value) = values.next() {
            out.push_sql("ROW(");
            BatchValue::walk_ast(&value.0, out.reborrow())?;
            out.push_sql(", ");
            BatchValue::walk_ast(&value.1, out.reborrow())?;
            out.push_sql(")");
        }
        for value in values {
            out.push_sql(", ROW(");
            BatchValue::walk_ast(&value.0, out.reborrow())?;
            out.push_sql(", ");
            BatchValue::walk_ast(&value.1, out.reborrow())?;
            out.push_sql(")");
        }
        out.push_sql(" )");

        // --- Set alias and its columns
        //
        //      AS tmp(column_a, column_b, column_c)
        out.push_sql(" AS ");
        out.push_identifier(Self::ALIAS)?;
        out.push_sql(" (");
        BatchColumn::walk_ast(&self.primary_key, out.reborrow())?; // p_key columns
        out.push_sql(", ");
        BatchColumn::walk_ast(&first.1, out.reborrow())?; // changeset columns
        out.push_sql(")");

        // --- Set equality condition for primary key(s)
        //
        // ON tab.column_b = tmp.column_b
        out.push_sql(" ON ");
        BatchColumnAssign::walk_ast(
            &self.primary_key,
            out.reborrow(),
            " AND ",
            false,
            Self::ALIAS,
        )?;

        // --- Assign target columns from temporary columns
        //
        //      tab.column_a = tmp.column_a,
        //      tab.column_c = tmp.column_c
        out.push_sql(" SET ");
        BatchColumnAssign::walk_ast(&first.1, out.reborrow(), ", ", false, Self::ALIAS)?;

        Ok(())
    }
}
