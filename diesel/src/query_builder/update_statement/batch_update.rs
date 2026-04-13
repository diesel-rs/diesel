use crate::backend::{Backend, sql_dialect};
use crate::expression::TypedExpressionType;
use crate::query_builder::{AstPass, QueryFragment};
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, SqlType};
use crate::{Column, QueryResult, Table, query_builder::*};
use core::marker::PhantomData;

/// Represents the column list for use in a batch update statement.
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
/// - `sep`: Separator that appears between consecutive assignments to alias.
/// - `ambiguous`: If the targeted column should contain leading table as identifier.
/// - `alias`: Alias name for the temporary table with identical column name.
///
///
/// #### Expected SQL fragments:
/// (Note: Identifier quotations omitted!)
///
/// * `BatchColumnAssign::walk_ast(&columns, out, ", ", false, "tmp")` \
///   = `"users.hair_color = tmp.hair_color, users.type = tmp.type"`
///
/// * `BatchColumnAssign::walk_ast(&columns, out, " AND ", true, "tmp")` \
///   = `"hair_color = tmp.hair_color AND type = tmp.type"`
pub trait BatchColumnAssign<Tab, DB: Backend> {
    /// The table these columns belong to
    type Table;

    /// Generate the SQL for the listed columns assignments.
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

/// Represents the value list for use in a batch update statement.
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

impl<T, ST, Tab, DB> BatchValue<ST, Tab, DB> for &T
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
// warn(dead_code) is a false positive for the fields 'values' and 'primary_key' as
// specialized implementations for the backends actually use them.
#[allow(dead_code)]
#[derive(Debug)]
pub struct BatchUpdate<I, C, PK, Tab> {
    // values.0 -> I: Identifier from Identifiable::Id
    // values.1 -> V: Changeset from AsChangeset::Changeset
    pub(crate) values: Vec<(I, C)>,
    // PK: PrimaryKey will have same SqlType as I
    pub(crate) primary_key: PK,
    _marker: PhantomData<Tab>,
}

impl<I, C, PK, Tab> BatchUpdate<I, C, PK, Tab> {
    /// Alias for temporary created table during batch update progress.
    pub const ALIAS: &str = "__diesel_internal_temp_values";

    /// Docs
    pub fn new(values: Vec<(I, C)>, primary_key: PK) -> Self {
        Self {
            values,
            primary_key,
            _marker: PhantomData,
        }
    }
}

impl<I, C, PK, Tab, DB> QueryFragment<DB> for BatchUpdate<I, C, PK, Tab>
where
    DB: Backend,
    DB::BatchUpdateSupport: sql_dialect::batch_update_support::SupportsBatchUpdate,
    Self: QueryFragment<DB, DB::BatchUpdateSupport>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::BatchUpdateSupport>>::walk_ast(self, pass)
    }
}
