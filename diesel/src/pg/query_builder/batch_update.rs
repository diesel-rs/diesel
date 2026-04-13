use crate::backend::{Backend, SqlDialect};
use crate::expression::TypedExpressionType;
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::EmptyChangeset;
use crate::result::Error::QueryBuilderError;
use crate::sql_types::{HasSqlType, SqlType};
use crate::{Expression, QueryResult, Table, query_builder::*};

impl<IT, CT, I, C, PK, Tab, DB>
    QueryFragment<DB, crate::pg::backend::PostgresLikeBatchUpdateSupport>
    for BatchUpdate<I, C, PK, Tab>
where
    DB: Backend
        + SqlDialect<BatchUpdateSupport = crate::pg::backend::PostgresLikeBatchUpdateSupport>,
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

        if self.values.is_empty() {
            return Err(QueryBuilderError(Box::new(EmptyChangeset)));
        }

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

        let first = self.values.first().expect("missing batch update values");

        // --- Assign target columns from temporary columns
        //
        //      column_a = tmp.column_a,
        //      column_c = tmp.column_c
        BatchColumnAssign::walk_ast(&first.1, out.reborrow(), ", ", true, Self::ALIAS)?;

        // --- List of values
        //
        // FROM ( VALUES
        //      ('aa', 1, 11),
        //      ('bb', 2, 22)
        // )
        out.push_sql(" FROM ( VALUES ");
        let mut values = self.values.iter();
        if let Some(value) = values.next() {
            out.push_sql("(");
            BatchValue::walk_ast(&value.0, out.reborrow())?;
            out.push_sql(", ");
            BatchValue::walk_ast(&value.1, out.reborrow())?;
            out.push_sql(")");
        }
        for value in values {
            out.push_sql(", (");
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
        out.push_sql("(");
        BatchColumn::walk_ast(&self.primary_key, out.reborrow())?; // p_key columns
        out.push_sql(", ");
        BatchColumn::walk_ast(&first.1, out.reborrow())?; // changeset columns
        out.push_sql(")");

        // --- Set equality condition for primary key(s)
        //
        // WHERE tab.column_b = tmp.column_b;
        out.push_sql(" WHERE ");
        BatchColumnAssign::walk_ast(
            &self.primary_key,
            out.reborrow(),
            " AND ",
            false,
            Self::ALIAS,
        )?;

        Ok(())
    }
}
