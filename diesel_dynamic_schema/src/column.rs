use core::borrow::Borrow;
use core::marker::PhantomData;
use diesel::backend::Backend;
use diesel::expression::{is_aggregate, TypedExpressionType, ValidGrouping};
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_source::ColumnHasTable;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// A database table column, created by [`Table::column`](crate::Table::column).
///
/// # Grouping and aggregate expressions
///
/// `ValidGrouping<GB>` holds for any `GB` with `IsAggregate = Never`, so mixing a runtime
/// column with an aggregate and no `.group_by()` compiles and leaves the verdict to the
/// server: PostgreSQL rejects it, MySQL rejects it only under
/// [`ONLY_FULL_GROUP_BY`](https://dev.mysql.com/doc/refman/8.4/en/sql-mode.html#sqlmode_only_full_group_by),
/// and SQLite answers from an [arbitrary row](https://www.sqlite.org/lang_select.html#bareagg).
pub struct Column<T, U, ST> {
    table: T,
    name: U,
    _sql_type: PhantomData<ST>,
}

impl<T, U, ST> Column<T, U, ST> {
    pub(crate) fn new(table: T, name: U) -> Self {
        Self {
            table,
            name,
            _sql_type: PhantomData,
        }
    }

    /// Gets a reference to the table of the column.
    pub fn table(&self) -> &T {
        &self.table
    }

    /// Gets the name of the column, as provided on creation.
    pub fn name(&self) -> &U {
        &self.name
    }
}

impl<T, U, ST> QueryId for Column<T, U, ST> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T, U, ST, QS> SelectableExpression<QS> for Column<T, U, ST> where Self: Expression {}

impl<T, U, ST, QS> AppearsOnTable<QS> for Column<T, U, ST> where Self: Expression {}

impl<T, U, ST> Expression for Column<T, U, ST>
where
    ST: TypedExpressionType,
{
    type SqlType = ST;
}

impl<T, U, ST, GB> ValidGrouping<GB> for Column<T, U, ST> {
    // `Never`, not `No`, because a blanket impl cannot coexist with a narrower
    // `ValidGrouping<()>`. Costs the mixed-aggregate compile error, see the type docs.
    type IsAggregate = is_aggregate::Never;
}

impl<T, U, ST, DB> QueryFragment<DB> for Column<T, U, ST>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: Borrow<str>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.table.walk_ast(out.reborrow())?;
        out.push_sql(".");
        out.push_identifier(self.name.borrow())?;
        Ok(())
    }
}

impl<T, U, ST> ColumnHasTable for Column<T, U, ST>
where
    U: Borrow<str>,
    T: Clone,
{
    type Table = T;

    fn table(&self) -> Self::Table {
        self.table.clone()
    }

    fn name(&self) -> &str {
        self.name.borrow()
    }
}
