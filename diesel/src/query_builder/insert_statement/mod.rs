mod batch_insert;
mod column_list;
mod insert_from_select;

pub use self::batch_insert::BatchInsert;
pub(crate) use self::column_list::ColumnList;
pub(crate) use self::insert_from_select::InsertFromSelect;

use std::marker::PhantomData;

use super::returning_clause::*;
use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::expression::grouped::Grouped;
use crate::expression::operators::Eq;
use crate::expression::{Expression, NonAggregate, SelectableExpression};
#[cfg(feature = "mysql")]
use crate::mysql::Mysql;
use crate::query_builder::*;
use crate::query_dsl::RunQueryDsl;
use crate::query_source::{Column, Table};
use crate::result::QueryResult;
#[cfg(feature = "sqlite")]
use crate::sqlite::Sqlite;
use crate::{insertable::*, QuerySource};

#[cfg(feature = "sqlite")]
mod insert_with_default_for_sqlite;

/// The structure returned by [`insert_into`].
///
/// The provided methods [`values`] and [`default_values`] will insert
/// data into the targeted table.
///
/// [`insert_into`]: crate::insert_into()
/// [`values`]: IncompleteInsertStatement::values()
/// [`default_values`]: IncompleteInsertStatement::default_values()
#[derive(Debug, Clone, Copy)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct IncompleteInsertStatement<T, Op> {
    target: T,
    operator: Op,
}

impl<T: QuerySource, Op> IncompleteInsertStatement<T, Op> {
    pub(crate) fn new(target: T, operator: Op) -> Self {
        IncompleteInsertStatement { target, operator }
    }

    /// Inserts `DEFAULT VALUES` into the targeted table.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users (name) {
    /// #         name -> Text,
    /// #         hair_color -> Text,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use self::users::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// connection.execute("CREATE TABLE users (
    ///     name VARCHAR(255) NOT NULL DEFAULT 'Sean',
    ///     hair_color VARCHAR(255) NOT NULL DEFAULT 'Green'
    /// )")?;
    ///
    /// insert_into(users)
    ///     .default_values()
    ///     .execute(connection)
    ///     .unwrap();
    /// let inserted_user = users.first(connection)?;
    /// let expected_data = (String::from("Sean"), String::from("Green"));
    ///
    /// assert_eq!(expected_data, inserted_user);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn default_values(self) -> InsertStatement<T, DefaultValues, Op> {
        self.values(DefaultValues)
    }

    /// Inserts the given values into the table passed to `insert_into`.
    ///
    /// See the documentation of [`insert_into`] for
    /// usage examples.
    ///
    /// This method can sometimes produce extremely opaque error messages due to
    /// limitations of the Rust language. If you receive an error about
    /// "overflow evaluating requirement" as a result of calling this method,
    /// you may need an `&` in front of the argument to this method.
    ///
    /// [`insert_into`]: crate::insert_into()
    pub fn values<U>(self, records: U) -> InsertStatement<T, U::Values, Op>
    where
        U: Insertable<T>,
    {
        InsertStatement::new(
            self.target,
            records.values(),
            self.operator,
            NoReturningClause,
        )
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// A fully constructed insert statement.
///
/// The parameters of this struct represent:
///
/// - `T`: The table we are inserting into
/// - `U`: The data being inserted
/// - `Op`: The operation being performed. The specific types used to represent
///   this are private, but correspond to SQL such as `INSERT` or `REPLACE`.
///   You can safely rely on the default type representing `INSERT`
/// - `Ret`: The `RETURNING` clause of the query. The specific types used to
///   represent this are private. You can safely rely on the default type
///   representing a query without a `RETURNING` clause.
pub struct InsertStatement<T: QuerySource, U, Op = Insert, Ret = NoReturningClause> {
    operator: Op,
    target: T,
    records: U,
    returning: Ret,
    into_clause: T::FromClause,
}

impl<T, U, Op, Ret> QueryId for InsertStatement<T, U, Op, Ret>
where
    T: QuerySource + QueryId + 'static,
    U: QueryId,
    Op: QueryId,
    Ret: QueryId,
{
    type QueryId = InsertStatement<T, U::QueryId, Op::QueryId, Ret::QueryId>;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID
        && U::HAS_STATIC_QUERY_ID
        && Op::HAS_STATIC_QUERY_ID
        && Ret::HAS_STATIC_QUERY_ID;
}

impl<T: QuerySource, U, Op, Ret> InsertStatement<T, U, Op, Ret> {
    fn new(target: T, records: U, operator: Op, returning: Ret) -> Self {
        InsertStatement {
            into_clause: target.from_clause(),
            operator,
            target,
            records,
            returning,
        }
    }

    pub(crate) fn replace_values<F, V>(self, f: F) -> InsertStatement<T, V, Op, Ret>
    where
        F: FnOnce(U) -> V,
    {
        InsertStatement::new(self.target, f(self.records), self.operator, self.returning)
    }
}

impl<T: QuerySource, U, C, Op, Ret> InsertStatement<T, InsertFromSelect<U, C>, Op, Ret> {
    /// Set the column list when inserting from a select statement
    ///
    /// See the documentation for [`insert_into`] for usage examples.
    ///
    /// [`insert_into`]: crate::insert_into()
    pub fn into_columns<C2>(
        self,
        columns: C2,
    ) -> InsertStatement<T, InsertFromSelect<U, C2>, Op, Ret>
    where
        C2: ColumnList<Table = T> + Expression,
        U: Query<SqlType = C2::SqlType>,
    {
        InsertStatement::new(
            self.target,
            self.records.with_columns(columns),
            self.operator,
            self.returning,
        )
    }
}

impl<T, U, Op, Ret, DB> QueryFragment<DB> for InsertStatement<T, U, Op, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB> + CanInsertInSingleQuery<DB>,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        if self.records.rows_to_insert() == Some(0) {
            out.push_sql("SELECT 1 FROM ");
            self.into_clause.walk_ast(out.reborrow())?;
            out.push_sql(" WHERE 1=0");
            return Ok(());
        }

        self.operator.walk_ast(out.reborrow())?;
        out.push_sql(" INTO ");
        self.into_clause.walk_ast(out.reborrow())?;
        out.push_sql(" ");
        self.records.walk_ast(out.reborrow())?;
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T, U, Op> AsQuery for InsertStatement<T, U, Op, NoReturningClause>
where
    T: Table,
    InsertStatement<T, U, Op, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = InsertStatement<T, U, Op, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<T, U, Op, Ret> Query for InsertStatement<T, U, Op, ReturningClause<Ret>>
where
    T: QuerySource,
    Ret: Expression + SelectableExpression<T> + NonAggregate,
{
    type SqlType = Ret::SqlType;
}

impl<T: QuerySource, U, Op, Ret, Conn> RunQueryDsl<Conn> for InsertStatement<T, U, Op, Ret> {}

impl<T: QuerySource, U, Op> InsertStatement<T, U, Op> {
    /// Specify what expression is returned after execution of the `insert`.
    /// # Examples
    ///
    /// ### Inserting records:
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let inserted_names = diesel::insert_into(users)
    ///     .values(&vec![name.eq("Timmy"), name.eq("Jimmy")])
    ///     .returning(name)
    ///     .get_results(connection);
    /// assert_eq!(Ok(vec!["Timmy".to_string(), "Jimmy".to_string()]), inserted_names);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> InsertStatement<T, U, Op, ReturningClause<E>>
    where
        InsertStatement<T, U, Op, ReturningClause<E>>: Query,
    {
        InsertStatement::new(
            self.target,
            self.records,
            self.operator,
            ReturningClause(returns),
        )
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
#[doc(hidden)]
pub struct Insert;

impl<DB: Backend> QueryFragment<DB> for Insert {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("INSERT");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
#[doc(hidden)]
pub struct InsertOrIgnore;

#[cfg(feature = "sqlite")]
impl QueryFragment<Sqlite> for InsertOrIgnore {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("INSERT OR IGNORE");
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<Mysql> for InsertOrIgnore {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("INSERT IGNORE");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
#[doc(hidden)]
pub struct Replace;

#[cfg(feature = "sqlite")]
impl QueryFragment<Sqlite> for Replace {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("REPLACE");
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<Mysql> for Replace {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("REPLACE");
        Ok(())
    }
}

/// Marker trait to indicate that no additional operations have been added
/// to a record for insert.
///
/// This is used to prevent things like
/// `.on_conflict_do_nothing().on_conflict_do_nothing()`
/// from compiling.
pub trait UndecoratedInsertRecord<Table> {}

impl<'a, T, Tab> UndecoratedInsertRecord<Tab> for &'a T where
    T: ?Sized + UndecoratedInsertRecord<Tab>
{
}

impl<T, U> UndecoratedInsertRecord<T::Table> for ColumnInsertValue<T, U> where T: Column {}

impl<T, U> UndecoratedInsertRecord<T::Table>
    for DefaultableColumnInsertValue<ColumnInsertValue<T, U>>
where
    T: Column,
{
}

impl<T, Table> UndecoratedInsertRecord<Table> for [T] where T: UndecoratedInsertRecord<Table> {}

impl<T, Table, QId, const STATIC_QUERY_ID: bool> UndecoratedInsertRecord<Table>
    for BatchInsert<T, Table, QId, STATIC_QUERY_ID>
where
    T: UndecoratedInsertRecord<Table>,
{
}

impl<T, Table> UndecoratedInsertRecord<Table> for Vec<T> where [T]: UndecoratedInsertRecord<Table> {}

impl<Lhs, Rhs> UndecoratedInsertRecord<Lhs::Table> for Eq<Lhs, Rhs> where Lhs: Column {}

impl<Lhs, Rhs, Tab> UndecoratedInsertRecord<Tab> for Option<Eq<Lhs, Rhs>> where
    Eq<Lhs, Rhs>: UndecoratedInsertRecord<Tab>
{
}

impl<Lhs, Rhs> UndecoratedInsertRecord<Lhs::Table> for Grouped<Eq<Lhs, Rhs>> where Lhs: Column {}

impl<Lhs, Rhs, Tab> UndecoratedInsertRecord<Tab> for Option<Grouped<Eq<Lhs, Rhs>>> where
    Eq<Lhs, Rhs>: UndecoratedInsertRecord<Tab>
{
}

impl<T, Table> UndecoratedInsertRecord<Table> for ValuesClause<T, Table> where
    T: UndecoratedInsertRecord<Table>
{
}

#[derive(Debug, Clone, Copy, QueryId)]
#[doc(hidden)]
pub struct DefaultValues;

impl<DB: Backend> CanInsertInSingleQuery<DB> for DefaultValues {
    fn rows_to_insert(&self) -> Option<usize> {
        Some(1)
    }
}

impl<Tab> Insertable<Tab> for DefaultValues {
    type Values = DefaultValues;

    fn values(self) -> Self::Values {
        self
    }
}

impl<'a, Tab> Insertable<Tab> for &'a DefaultValues {
    type Values = DefaultValues;

    fn values(self) -> Self::Values {
        *self
    }
}

impl<DB> QueryFragment<DB> for DefaultValues
where
    DB: Backend,
    Self: QueryFragment<DB, DB::DefaultValueClauseForInsert>,
{
    fn walk_ast<'a, 'b>(&'a self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        <Self as QueryFragment<DB, DB::DefaultValueClauseForInsert>>::walk_ast(self, pass)
    }
}

impl<DB> QueryFragment<DB, sql_dialect::default_value_clause::AnsiDefaultValueClause>
    for DefaultValues
where
    DB: Backend
        + SqlDialect<
            DefaultValueClauseForInsert = sql_dialect::default_value_clause::AnsiDefaultValueClause,
        >,
{
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("DEFAULT VALUES");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct ValuesClause<T, Tab> {
    pub values: T,
    _marker: PhantomData<Tab>,
}

impl<T: Default, Tab> Default for ValuesClause<T, Tab> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T, Tab> ValuesClause<T, Tab> {
    pub(crate) fn new(values: T) -> Self {
        Self {
            values,
            _marker: PhantomData,
        }
    }
}

impl<T, Tab, DB> CanInsertInSingleQuery<DB> for ValuesClause<T, Tab>
where
    DB: Backend,
    T: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        self.values.rows_to_insert()
    }
}

impl<T, Tab, DB> QueryFragment<DB> for ValuesClause<T, Tab>
where
    DB: Backend,
    Tab: Table,
    T: InsertValues<Tab, DB>,
    DefaultValues: QueryFragment<DB>,
{
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        if self.values.is_noop()? {
            DefaultValues.walk_ast(out)?;
        } else {
            out.push_sql("(");
            self.values.column_names(out.reborrow())?;
            out.push_sql(") VALUES (");
            self.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}
