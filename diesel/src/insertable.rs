use std::marker::PhantomData;

use backend::{Backend, SupportsDefaultKeyword};
use expression::{AppearsOnTable, Expression};
use query_builder::{
    AstPass, InsertStatement, QueryFragment, UndecoratedInsertRecord, ValuesClause,
};
use query_source::{Column, Table};
use result::QueryResult;
#[cfg(feature = "sqlite")]
use sqlite::Sqlite;

/// Represents that a structure can be used to insert a new row into the
/// database. This is automatically implemented for `&[T]` and `&Vec<T>` for
/// inserting more than one record.
///
/// ### Deriving
///
/// This trait can be automatically derived by adding  `#[derive(Insertable)]`
/// to your struct. Structs which derive this trait must also be annotated
/// with `#[table_name = "some_table_name"]`. If the field name of your
/// struct differs from the name of the column, you can annotate the field
/// with `#[column_name = "some_column_name"]`.
///
/// Your struct can also contain fields which implement `Insertable`. This is
/// useful when you want to have one field map to more than one column (for
/// example, an enum that maps to a label and a value column). Add
/// `#[diesel(embed)]` to any such fields.
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::{posts, users};
    /// #     let conn = establish_connection();
    /// #     diesel::delete(posts::table).execute(&conn)?;
    /// users::table
    ///     .select((
    ///         users::name.concat("'s First Post"),
    ///         users::id,
    ///     ))
    ///     .insert_into(posts::table)
    ///     .into_columns((posts::title, posts::user_id))
    ///     .execute(&conn)?;
    ///
    /// let inserted_posts = posts::table
    ///     .select(posts::title)
    ///     .load::<String>(&conn)?;
    /// let expected = vec!["Sean's First Post", "Tess's First Post"];
    /// assert_eq!(expected, inserted_posts);
    /// #     Ok(())
    /// # }
    /// ```
    fn insert_into(self, table: T) -> InsertStatement<T, Self::Values>
    where
        Self: Sized,
    {
        ::insert_into(table).values(self)
    }
}

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

impl<'a, T, Tab, DB> CanInsertInSingleQuery<DB> for BatchInsert<'a, T, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.records.len())
    }
}

impl<T, Table, DB> CanInsertInSingleQuery<DB> for OwnedBatchInsert<T, Table>
where
    DB: Backend + SupportsDefaultKeyword,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.values.len())
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

pub trait InsertValues<T: Table, DB: Backend>: QueryFragment<DB> {
    fn column_names(&self, out: AstPass<DB>) -> QueryResult<()>;
}

#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub enum ColumnInsertValue<Col, Expr> {
    Expression(Col, Expr),
    Default,
}

impl<Col, Expr> Default for ColumnInsertValue<Col, Expr> {
    fn default() -> Self {
        ColumnInsertValue::Default
    }
}

impl<Col, Expr, DB> InsertValues<Col::Table, DB> for ColumnInsertValue<Col, Expr>
where
    DB: Backend + SupportsDefaultKeyword,
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<()>,
    Self: QueryFragment<DB>,
{
    fn column_names(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(Col::NAME)?;
        Ok(())
    }
}

impl<Col, Expr, DB> QueryFragment<DB> for ColumnInsertValue<Col, Expr>
where
    DB: Backend + SupportsDefaultKeyword,
    Expr: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(_, ref value) = *self {
            value.walk_ast(out.reborrow())?;
        } else {
            out.push_sql("DEFAULT");
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<Col, Expr> InsertValues<Col::Table, Sqlite> for ColumnInsertValue<Col, Expr>
where
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<()>,
    Self: QueryFragment<Sqlite>,
{
    fn column_names(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(..) = *self {
            out.push_identifier(Col::NAME)?;
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<Col, Expr> QueryFragment<Sqlite> for ColumnInsertValue<Col, Expr>
where
    Expr: QueryFragment<Sqlite>,
{
    fn walk_ast(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(_, ref value) = *self {
            value.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a [T]
where
    &'a T: UndecoratedInsertRecord<Tab>,
{
    type Values = BatchInsert<'a, T, Tab>;

    fn values(self) -> Self::Values {
        BatchInsert {
            records: self,
            _marker: PhantomData,
        }
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
    type Values = OwnedBatchInsert<T::Values, Tab>;

    fn values(self) -> Self::Values {
        OwnedBatchInsert {
            values: self.into_iter().map(Insertable::values).collect(),
            _marker: PhantomData,
        }
    }
}

impl<T, Tab> Insertable<Tab> for Option<T>
where
    T: Insertable<Tab>,
    T::Values: Default,
{
    type Values = T::Values;

    fn values(self) -> Self::Values {
        self.map(Insertable::values).unwrap_or_default()
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

#[derive(Debug, Clone, Copy)]
pub struct BatchInsert<'a, T: 'a, Tab> {
    pub records: &'a [T],
    _marker: PhantomData<Tab>,
}

impl<'a, T, Tab, Inner, DB> QueryFragment<DB> for BatchInsert<'a, T, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
    &'a T: Insertable<Tab, Values = ValuesClause<Inner, Tab>>,
    ValuesClause<Inner, Tab>: QueryFragment<DB>,
    Inner: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        let mut records = self.records.iter().map(Insertable::values);
        if let Some(record) = records.next() {
            record.walk_ast(out.reborrow())?;
        }
        for record in records {
            out.push_sql(", (");
            record.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct OwnedBatchInsert<V, Tab> {
    pub values: Vec<V>,
    _marker: PhantomData<Tab>,
}

impl<Tab, DB, Inner> QueryFragment<DB> for OwnedBatchInsert<ValuesClause<Inner, Tab>, Tab>
where
    DB: Backend + SupportsDefaultKeyword,
    ValuesClause<Inner, Tab>: QueryFragment<DB>,
    Inner: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        let mut values = self.values.iter();
        if let Some(value) = values.next() {
            value.walk_ast(out.reborrow())?;
        }
        for value in values {
            out.push_sql(", (");
            value.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}
