use crate::backend::{Backend, SqlDialect, sql_dialect};
use crate::expression::TypedExpressionType;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::result::EmptyChangeset;
use crate::result::Error::QueryBuilderError;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, SqlType};
use crate::{Column, Expression, QueryResult, Table, query_builder::*};
use std::marker::PhantomData;

/// Represents the column list for use in an batch update statement.
///
/// This trait is implemented by columns and tuples of columns.
pub trait BatchColumn<Tab, DB: Backend> {
    /// The table these columns belong to
    type Table;

    /// Generate the SQL for this columns list.
    ///
    /// Column names must *not* be qualified.
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;
}

impl<C, Tab, DB> BatchColumn<Tab, DB> for C
where
    C: Column,
    Tab: Table,
    DB: Backend,
{
    type Table = Tab;

    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_identifier(C::NAME)?;
        Ok(())
    }
}

/// Represents the listed columns assigned to alias columns for use in an
/// batch update statement.
///
/// This trait is implemented by columns and tuples of columns.
///
/// - `sep`: Seperator that appears between consecutive asignments to alias.
/// - `ambiguous`: If the targeted column should contain leading table as identifier.
/// - `alias`: Alias name for the temporary table with identical column name.
///
///
/// #### Expected SQL fragments:
/// (Note: Identifier quotations omitted!)
///
/// * `BatchColumnAssign::walk_ast(&columns, out, ", ", false, "tmp")` \
/// = `"users.hair_color = tmp.hair_color, users.type = tmp.type"`
///
/// * `BatchColumnAssign::walk_ast(&columns, out, " AND ", true, "tmp")` \
/// = `"hair_color = tmp.hair_color AND type = tmp.type"`
pub trait BatchColumnAssign<Tab, DB: Backend> {
    /// The table these columns belong to
    type Table;

    /// Generate the SQL for this columns list.
    ///
    /// Column names must *not* be qualified.
    fn walk_ast<'b>(
        &'b self,
        out: AstPass<'_, 'b, DB>,
        sep: &'_ str,
        ambiguous: bool,
        alias: &'_ str,
    ) -> QueryResult<()>;
}

impl<C, Tab, DB> BatchColumnAssign<Tab, DB> for C
where
    C: Column + QueryFragment<DB>,
    Tab: Table,
    DB: Backend,
{
    type Table = Tab;

    fn walk_ast<'b>(
        &'b self,
        mut out: AstPass<'_, 'b, DB>,
        _sep: &'_ str,
        ambiguous: bool,
        alias: &'_ str,
    ) -> QueryResult<()> {
        match ambiguous {
            true => out.push_identifier(<C as Column>::NAME)?,
            false => <C as QueryFragment<DB>>::walk_ast(self, out.reborrow())?,
        }
        out.push_sql(" = ");
        out.push_identifier(alias)?;
        out.push_sql(".");
        out.push_identifier(<C as Column>::NAME)?;
        Ok(())
    }
}

/// Represents the value list for use in an batch update statement.
///
/// This trait is implemented by all referenced values that also implement [ToSql],
/// [crate::query_builder::update_statement::changeset::Assign] and tuples of both.
pub trait BatchValue<ST, Tab, DB: Backend> {
    /// The table these values belong to
    type Table;
    /// The SqlType these values refer to
    type SqlType;

    /// Generate the SQL for this value list.
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()>;
}

impl<'a, T, ST, Tab, DB> BatchValue<ST, Tab, DB> for &'a T
where
    T: ToSql<ST, DB>,
    ST: SqlType + TypedExpressionType,
    Tab: Table,
    DB: Backend + HasSqlType<ST>,
{
    type Table = Tab;
    type SqlType = ST;

    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_bind_param(self)?;
        Ok(())
    }
}

/// This type represents a batch update clause, which allows
/// to update multiple rows at once.
///
/// Custom backends can specialize the [`QueryFragment`]
/// implementation via [`SqlDialect::BatchUpdateSupport`]
/// or provide fully custom [`ExecuteDsl`](crate::query_dsl::methods::ExecuteDsl)
/// and [`LoadQuery`](crate::query_dsl::methods::LoadQuery) implementations
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
#[derive(Debug)]
pub struct BatchUpdate<I, C, PK, Tab, QId, const STABLE_QUERY_ID: bool> {
    // values.0 -> I: Identifier from Identifiable::Id
    // values.1 -> V: Changeset from AsChangeset::Changeset
    pub(crate) values: Vec<(I, C)>,
    // PK: PrimaryKey will have same SqlType as I
    pub(crate) primary_key: PK,
    _marker: PhantomData<(Tab, QId)>,
}

impl<I, C, PK, Tab, QId, const STABLE_QUERY_ID: bool>
    BatchUpdate<I, C, PK, Tab, QId, STABLE_QUERY_ID>
{
    const ALIAS: &str = "__diesel_internal_temp_values";

    /// Docs
    pub fn new(values: Vec<(I, C)>, primary_key: PK) -> Self {
        Self {
            values,
            primary_key,
            _marker: PhantomData,
        }
    }
}

impl<I, C, PK, Tab: 'static, QId: 'static, const STABLE_QUERY_ID: bool> QueryId
    for BatchUpdate<I, C, PK, Tab, QId, STABLE_QUERY_ID>
{
    type QueryId = QId;

    const HAS_STATIC_QUERY_ID: bool = STABLE_QUERY_ID;
}

impl<I, C, PK, Tab, QId, const HAS_STATIC_QUERY_ID: bool, DB> QueryFragment<DB>
    for BatchUpdate<I, C, PK, Tab, QId, HAS_STATIC_QUERY_ID>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::BatchUpdateSupport>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::BatchUpdateSupport>>::walk_ast(self, pass)
    }
}

/// TODO:
/// Does it make sense to put this to pg/query_builder
impl<IST, CST, I, C, PK, Tab, DB, QId, const STABLE_QUERY_ID: bool>
    QueryFragment<DB, sql_dialect::batch_update_support::PostgresLikeBatchUpdateSupport>
    for BatchUpdate<I, C, PK, Tab, QId, STABLE_QUERY_ID>
where
    DB: Backend
        + SqlDialect<
            BatchUpdateSupport = sql_dialect::batch_update_support::PostgresLikeBatchUpdateSupport,
        >,
    IST: SqlType + TypedExpressionType, // SqlType of I
    CST: SqlType + TypedExpressionType, // SqlType of  C
    I: BatchValue<IST, Tab, DB>,
    C: Expression<SqlType = CST>
        + BatchColumn<Tab, DB>
        + BatchColumnAssign<Tab, DB>
        + BatchValue<CST, Tab, DB>,
    PK: Expression<SqlType = IST> + BatchColumnAssign<Tab, DB> + BatchColumn<Tab, DB>,
    Tab: Table<PrimaryKey = PK>,
    DB: Backend + HasSqlType<IST> + HasSqlType<CST>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
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

        let first = self.values.first().unwrap();

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

/// TODO:
/// Does it make sense to put this to mysql/query_builder
impl<IST, CST, I, C, PK, Tab, DB, QId, const STABLE_QUERY_ID: bool>
    QueryFragment<DB, sql_dialect::batch_update_support::MySqlLikeBatchUpdateSupport>
    for BatchUpdate<I, C, PK, Tab, QId, STABLE_QUERY_ID>
where
    DB: Backend
        + SqlDialect<
            BatchUpdateSupport = sql_dialect::batch_update_support::MySqlLikeBatchUpdateSupport,
        >,
    IST: SqlType + TypedExpressionType, // SqlType of I
    CST: SqlType + TypedExpressionType, // SqlType of  C
    I: BatchValue<IST, Tab, DB>,
    C: Expression<SqlType = CST>
        + BatchColumn<Tab, DB>
        + BatchColumnAssign<Tab, DB>
        + BatchValue<CST, Tab, DB>,
    PK: Expression<SqlType = IST> + BatchColumnAssign<Tab, DB> + BatchColumn<Tab, DB>,
    Tab: Table<PrimaryKey = PK>,
    DB: Backend + HasSqlType<IST> + HasSqlType<CST>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        // TODO:
        // UpdateStatement::walk_ast(...) already added " SET " to the sql.
        // MySql does not allow a temporary table definition or a join right
        // after the keyword 'SET' and its assignments.
        // Return Error by default due to missing support.
        // return Err(QueryBuilderError(Box::new(EmptyChangeset)));

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

        // Implementation for Option 2, that works, when commenting out the line
        // `out.push_sql(" SET ");` from UpdateStatement::walk_ast(...).

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

        let first = self.values.first().unwrap();

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

/// TODO:
/// Check out if there is a supported batch update syntax for sqlite.
impl<IST, CST, I, C, PK, Tab, DB, QId, const STABLE_QUERY_ID: bool>
    QueryFragment<DB, sql_dialect::batch_update_support::DoesNotSupportBatchUpdate>
    for BatchUpdate<I, C, PK, Tab, QId, STABLE_QUERY_ID>
where
    DB: Backend
        + SqlDialect<
            BatchUpdateSupport = sql_dialect::batch_update_support::DoesNotSupportBatchUpdate,
        >,
    IST: SqlType + TypedExpressionType, // SqlType of I
    CST: SqlType + TypedExpressionType, // SqlType of  C
    I: BatchValue<IST, Tab, DB>,
    C: Expression<SqlType = CST>
        + BatchColumn<Tab, DB>
        + BatchColumnAssign<Tab, DB>
        + BatchValue<CST, Tab, DB>,
    PK: Expression<SqlType = IST> + BatchColumnAssign<Tab, DB> + BatchColumn<Tab, DB>,
    Tab: Table<PrimaryKey = PK>,
    DB: Backend + HasSqlType<IST> + HasSqlType<CST>,
{
    fn walk_ast<'b>(&'b self, mut _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Err(QueryBuilderError(Box::new(EmptyChangeset)))
        // Ok(())
    }
}
