use backend::{Backend, SupportsDefaultKeyword};
use expression::{AppearsOnTable, Expression};
use expression::operators::Eq;
use result::QueryResult;
use query_builder::{AstPass, QueryBuilder, QueryFragment};
use query_builder::insert_statement::UndecoratedInsertRecord;
use query_source::{Column, Table};
#[cfg(feature = "sqlite")]
use sqlite::Sqlite;

/// Represents that a structure can be used to insert a new row into the
/// database. This is automatically implemented for `&[T]` and `&Vec<T>` for
/// inserting more than one record.
///
/// ### Deriving
///
/// This trait can be automatically derived using `diesel_codegen` by adding
/// `#[derive(Insertable)]` to your struct. Structs which derive this trait must
/// also be annotated with `#[table_name = "some_table_name"]`. If the field
/// name of your struct differs from the name of the column, you can annotate
/// the field with `#[column_name = "some_column_name"]`.
pub trait Insertable<T: Table, DB: Backend> {
    type Values: InsertValues<T, DB>;

    fn values(self) -> Self::Values;
}

pub trait CanInsertInSingleQuery<DB: Backend> {
    fn rows_to_insert(&self) -> usize;
}

impl<'a, T, DB> CanInsertInSingleQuery<DB> for &'a T
where
    T: ?Sized + CanInsertInSingleQuery<DB>,
    DB: Backend,
{
    fn rows_to_insert(&self) -> usize {
        (*self).rows_to_insert()
    }
}

pub trait InsertValues<T: Table, DB: Backend> {
    fn column_names(&self, out: &mut DB::QueryBuilder) -> QueryResult<()>;
    fn walk_ast(&self, out: AstPass<DB>) -> QueryResult<()>;

    /// Whether or not `column_names` and `walk_ast` will perform any action
    ///
    /// This method will return `true` for values which semantically represent
    /// `DEFAULT` on backends which don't support it, or `DEFAULT VALUES` on
    /// any backend.
    ///
    /// Note: This method only has semantic meaning for types which represent a
    /// single row. Types which represent multiple rows will always return
    /// `false` for this, even if they will insert 0 rows.
    fn is_noop(&self) -> bool;
}

#[derive(Debug, Copy, Clone)]
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
    Expr: Expression<SqlType = Col::SqlType> + QueryFragment<DB> + AppearsOnTable<()>,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> QueryResult<()> {
        out.push_identifier(Col::NAME)?;
        Ok(())
    }

    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(_, ref value) = *self {
            value.walk_ast(out.reborrow())?;
        } else {
            out.push_sql("DEFAULT");
        }
        Ok(())
    }

    fn is_noop(&self) -> bool {
        false
    }
}

#[cfg(feature = "sqlite")]
impl<Col, Expr> InsertValues<Col::Table, Sqlite> for ColumnInsertValue<Col, Expr>
where
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + QueryFragment<Sqlite> + AppearsOnTable<()>,
{
    fn column_names(&self, out: &mut <Sqlite as Backend>::QueryBuilder) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(..) = *self {
            out.push_identifier(Col::NAME)?;
        }
        Ok(())
    }

    fn walk_ast(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
        if let ColumnInsertValue::Expression(_, ref value) = *self {
            value.walk_ast(out.reborrow())?;
        }
        Ok(())
    }

    fn is_noop(&self) -> bool {
        if let ColumnInsertValue::Expression(..) = *self {
            false
        } else {
            true
        }
    }
}

impl<'a, T, Tab, DB> Insertable<Tab, DB> for &'a [T]
where
    Tab: Table,
    DB: Backend,
    BatchInsertValues<'a, T>: InsertValues<Tab, DB>,
    &'a T: UndecoratedInsertRecord<Tab>,
{
    type Values = BatchInsertValues<'a, T>;

    fn values(self) -> Self::Values {
        BatchInsertValues { records: self }
    }
}

impl<T, DB> CanInsertInSingleQuery<DB> for [T]
where
    DB: Backend + SupportsDefaultKeyword,
    T: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> usize {
        self.len()
    }
}

impl<'a, T, Tab, DB> Insertable<Tab, DB> for &'a Vec<T>
where
    Tab: Table,
    DB: Backend,
    &'a [T]: Insertable<Tab, DB>,
{
    type Values = <&'a [T] as Insertable<Tab, DB>>::Values;

    fn values(self) -> Self::Values {
        (&**self).values()
    }
}

impl<T, DB> CanInsertInSingleQuery<DB> for Vec<T>
where
    DB: Backend,
    [T]: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> usize {
        self.as_slice().rows_to_insert()
    }
}

impl<Lhs, Rhs, DB> CanInsertInSingleQuery<DB> for Eq<Lhs, Rhs>
where
    DB: Backend,
{
    fn rows_to_insert(&self) -> usize {
        1
    }
}

impl<T, DB> CanInsertInSingleQuery<DB> for Option<T>
where
    DB: Backend,
    T: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> usize {
        if let Some(ref value) = *self {
            debug_assert_eq!(value.rows_to_insert(), 1);
        }
        1
    }
}

impl<'a, T, Tab, DB> Insertable<Tab, DB> for &'a Option<T>
where
    Tab: Table,
    DB: Backend,
    &'a T: Insertable<Tab, DB>,
    <&'a T as Insertable<Tab, DB>>::Values: Default,
{
    type Values = <&'a T as Insertable<Tab, DB>>::Values;

    fn values(self) -> Self::Values {
        match *self {
            Some(ref v) => v.values(),
            None => Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchInsertValues<'a, T: 'a> {
    records: &'a [T],
}

impl<'a, T, Tab, DB> InsertValues<Tab, DB> for BatchInsertValues<'a, T>
where
    Tab: Table,
    DB: Backend + SupportsDefaultKeyword,
    &'a T: Insertable<Tab, DB>,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> QueryResult<()> {
        self.records
            .get(0)
            .expect("Tried to read column names from empty list of rows")
            .values()
            .column_names(out)
    }

    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        for (i, record) in self.records.iter().enumerate() {
            if i != 0 {
                out.push_sql(", ");
            }
            record.values().walk_ast(out.reborrow())?;
        }
        Ok(())
    }

    fn is_noop(&self) -> bool {
        false
    }
}
