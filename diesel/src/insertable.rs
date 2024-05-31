use std::marker::PhantomData;

use crate::backend::{sql_dialect, Backend, DieselReserveSpecialization, SqlDialect};
use crate::expression::grouped::Grouped;
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::{
    AstPass, BatchInsert, InsertStatement, NoFromClause, QueryFragment, QueryId,
    UndecoratedInsertRecord, ValuesClause,
};
use crate::query_source::{Column, Table};
use crate::result::QueryResult;

/// Represents that a structure can be used to insert a new row into the
/// database. This is automatically implemented for `&[T]` and `&Vec<T>` for
/// inserting more than one record.
///
/// This trait can be [derived](derive@Insertable)
pub trait Insertable<T> {
    /// The `VALUES` clause to insert these records
    ///
    /// The types used here are generally internal to Diesel.
    /// Implementations of this trait should use the `Values`
    /// type of other `Insertable` types.
    /// For example `<diesel::dsl::Eq<column, &str> as Insertable<table>>::Values`.
    type Values;

    /// Construct `Self::Values`
    ///
    /// Implementations of this trait typically call `.values`
    /// on other `Insertable` types.
    fn values(self) -> Self::Values;

    /// Insert `self` into a given table.
    ///
    /// `foo.insert_into(table)` is identical to `insert_into(table).values(foo)`.
    /// However, when inserting from a select statement,
    /// this form is generally preferred.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::{posts, users};
    /// #     let conn = &mut establish_connection();
    /// #     diesel::delete(posts::table).execute(conn)?;
    /// users::table
    ///     .select((
    ///         users::name.concat("'s First Post"),
    ///         users::id,
    ///     ))
    ///     .insert_into(posts::table)
    ///     .into_columns((posts::title, posts::user_id))
    ///     .execute(conn)?;
    ///
    /// let inserted_posts = posts::table
    ///     .select(posts::title)
    ///     .load::<String>(conn)?;
    /// let expected = vec!["Sean's First Post", "Tess's First Post"];
    /// assert_eq!(expected, inserted_posts);
    /// #     Ok(())
    /// # }
    /// ```
    fn insert_into(self, table: T) -> InsertStatement<T, Self::Values>
    where
        T: Table,
        Self: Sized,
    {
        crate::insert_into(table).values(self)
    }
}

#[doc(inline)]
pub use diesel_derives::Insertable;

pub trait CanInsertInSingleQuery<DB: Backend> {
    /// How many rows will this query insert?
    ///
    /// This function should only return `None` when the query is valid on all
    /// backends, regardless of how many rows get inserted.
    fn rows_to_insert(&self) -> Option<usize>;
}

impl<'a, T, DB> CanInsertInSingleQuery<DB> for &'a T
where
    T: ?Sized + CanInsertInSingleQuery<DB>,
    DB: Backend,
{
    fn rows_to_insert(&self) -> Option<usize> {
        (*self).rows_to_insert()
    }
}

impl<T, U, DB> CanInsertInSingleQuery<DB> for ColumnInsertValue<T, U>
where
    DB: Backend,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(1)
    }
}

impl<V, DB> CanInsertInSingleQuery<DB> for DefaultableColumnInsertValue<V>
where
    DB: Backend,
    V: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(1)
    }
}

pub trait InsertValues<DB: Backend, T: Table>: QueryFragment<DB> {
    fn column_names(&self, out: AstPass<'_, '_, DB>) -> QueryResult<()>;
}

#[derive(Debug, Copy, Clone, QueryId)]
#[doc(hidden)]
pub struct ColumnInsertValue<Col, Expr> {
    pub(crate) expr: Expr,
    p: PhantomData<Col>,
}

impl<Col, Expr> ColumnInsertValue<Col, Expr> {
    pub(crate) fn new(expr: Expr) -> Self {
        Self {
            expr,
            p: PhantomData,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub enum DefaultableColumnInsertValue<T> {
    Expression(T),
    Default,
}

impl<T> QueryId for DefaultableColumnInsertValue<T> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

#[allow(clippy::derivable_impls)] // that's not supported on rust 1.65
impl<T> Default for DefaultableColumnInsertValue<T> {
    fn default() -> Self {
        DefaultableColumnInsertValue::Default
    }
}

impl<Col, Expr, DB> InsertValues<DB, Col::Table>
    for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    DB: Backend + SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>,
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<NoFromClause>,
    Self: QueryFragment<DB>,
{
    fn column_names(&self, mut out: AstPass<'_, '_, DB>) -> QueryResult<()> {
        out.push_identifier(Col::NAME)?;
        Ok(())
    }
}

impl<Col, Expr, DB> InsertValues<DB, Col::Table> for ColumnInsertValue<Col, Expr>
where
    DB: Backend,
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<NoFromClause>,
    Self: QueryFragment<DB>,
{
    fn column_names(&self, mut out: AstPass<'_, '_, DB>) -> QueryResult<()> {
        out.push_identifier(Col::NAME)?;
        Ok(())
    }
}

impl<Expr, DB> QueryFragment<DB> for DefaultableColumnInsertValue<Expr>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::InsertWithDefaultKeyword>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::InsertWithDefaultKeyword>>::walk_ast(self, pass)
    }
}

impl<Expr, DB> QueryFragment<DB, sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword> for DefaultableColumnInsertValue<Expr>
where
    DB: Backend + SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        if let Self::Expression(ref inner) = *self {
            inner.walk_ast(out.reborrow())?;
        } else {
            out.push_sql("DEFAULT");
        }
        Ok(())
    }
}

impl<Col, Expr, DB> QueryFragment<DB> for ColumnInsertValue<Col, Expr>
where
    DB: Backend + DieselReserveSpecialization,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.expr.walk_ast(pass)
    }
}

#[cfg(feature = "sqlite")]
impl<Col, Expr> InsertValues<crate::sqlite::Sqlite, Col::Table>
    for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<NoFromClause>,
    Self: QueryFragment<crate::sqlite::Sqlite>,
{
    fn column_names(&self, mut out: AstPass<'_, '_, crate::sqlite::Sqlite>) -> QueryResult<()> {
        if let Self::Expression(..) = *self {
            out.push_identifier(Col::NAME)?;
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<Col, Expr>
    QueryFragment<
        crate::sqlite::Sqlite,
        crate::backend::sql_dialect::default_keyword_for_insert::DoesNotSupportDefaultKeyword,
    > for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    Expr: QueryFragment<crate::sqlite::Sqlite>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        if let Self::Expression(ref inner) = *self {
            inner.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a [T]
where
    &'a T: UndecoratedInsertRecord<Tab> + Insertable<Tab>,
{
    type Values = BatchInsert<Vec<<&'a T as Insertable<Tab>>::Values>, Tab, (), false>;

    fn values(self) -> Self::Values {
        let values = self.iter().map(Insertable::values).collect::<Vec<_>>();
        BatchInsert::new(values)
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a Vec<T>
where
    &'a [T]: Insertable<Tab>,
{
    type Values = <&'a [T] as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        (&**self).values()
    }
}

impl<T, Tab> Insertable<Tab> for Vec<T>
where
    T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
    type Values = BatchInsert<Vec<T::Values>, Tab, (), false>;

    fn values(self) -> Self::Values {
        let values = self.into_iter().map(Insertable::values).collect::<Vec<_>>();
        BatchInsert::new(values)
    }
}

impl<T, Tab, const N: usize> Insertable<Tab> for [T; N]
where
    T: Insertable<Tab>,
{
    type Values = BatchInsert<Vec<T::Values>, Tab, [T::Values; N], true>;

    // We must use the deprecated `IntoIter` function
    // here as 1.51 (MSRV) does not support the new not
    // deprecated variant
    #[allow(deprecated)]
    fn values(self) -> Self::Values {
        let values = std::array::IntoIter::new(self)
            .map(Insertable::values)
            .collect::<Vec<_>>();
        BatchInsert::new(values)
    }
}

impl<'a, T, Tab, const N: usize> Insertable<Tab> for &'a [T; N]
where
    T: Insertable<Tab>,
    &'a T: Insertable<Tab>,
{
    // We can reuse the query id for [T; N] here as this
    // compiles down to the same query
    type Values = BatchInsert<Vec<<&'a T as Insertable<Tab>>::Values>, Tab, [T::Values; N], true>;

    fn values(self) -> Self::Values {
        let values = self.iter().map(Insertable::values).collect();
        BatchInsert::new(values)
    }
}

impl<T, Tab, const N: usize> Insertable<Tab> for Box<[T; N]>
where
    T: Insertable<Tab>,
{
    // We can reuse the query id for [T; N] here as this
    // compiles down to the same query
    type Values = BatchInsert<Vec<T::Values>, Tab, [T::Values; N], true>;

    fn values(self) -> Self::Values {
        let v = Vec::from(self as Box<[T]>);
        let values = v.into_iter().map(Insertable::values).collect::<Vec<_>>();
        BatchInsert::new(values)
    }
}

mod private {
    // This helper exists to differentiate between
    // Insertable implementations for tuples and for single values
    #[allow(missing_debug_implementations)]
    pub struct InsertableOptionHelper<T, V>(
        pub(crate) Option<T>,
        pub(crate) std::marker::PhantomData<V>,
    );
}

pub(crate) use self::private::InsertableOptionHelper;

impl<T, Tab, V> Insertable<Tab> for Option<T>
where
    T: Insertable<Tab, Values = ValuesClause<V, Tab>>,
    InsertableOptionHelper<T, V>: Insertable<Tab>,
{
    type Values = <InsertableOptionHelper<T, V> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        InsertableOptionHelper(self, PhantomData).values()
    }
}

impl<T, Tab, Expr, Col> Insertable<Tab> for InsertableOptionHelper<T, ColumnInsertValue<Col, Expr>>
where
    T: Insertable<Tab, Values = ValuesClause<ColumnInsertValue<Col, Expr>, Tab>>,
{
    type Values = ValuesClause<DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>, Tab>;

    fn values(self) -> Self::Values {
        ValuesClause::new(
            self.0
                .map(|v| DefaultableColumnInsertValue::Expression(Insertable::values(v).values))
                .unwrap_or_default(),
        )
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a Option<T>
where
    Option<&'a T>: Insertable<Tab>,
{
    type Values = <Option<&'a T> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        self.as_ref().values()
    }
}

impl<L, R, Tab> Insertable<Tab> for Grouped<crate::expression::operators::Eq<L, R>>
where
    crate::expression::operators::Eq<L, R>: Insertable<Tab>,
{
    type Values = <crate::expression::operators::Eq<L, R> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        self.0.values()
    }
}

impl<'a, L, R, Tab> Insertable<Tab> for &'a Grouped<crate::expression::operators::Eq<L, R>>
where
    &'a crate::expression::operators::Eq<L, R>: Insertable<Tab>,
{
    type Values = <&'a crate::expression::operators::Eq<L, R> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        self.0.values()
    }
}
