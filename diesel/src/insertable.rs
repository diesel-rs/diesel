use backend::{Backend, SupportsDefaultKeyword};
use expression::{AppearsOnTable, Expression};
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
pub trait Insertable<T> {
    type Values;

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

impl<T, DB> CanInsertInSingleQuery<DB> for [T]
where
    DB: Backend + SupportsDefaultKeyword,
{
    fn rows_to_insert(&self) -> usize {
        self.len()
    }
}

impl<T, U, DB> CanInsertInSingleQuery<DB> for ColumnInsertValue<T, U>
where
    DB: Backend,
{
    fn rows_to_insert(&self) -> usize {
        1
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

    // FIXME: Once #1166 is done we should just wrap the value in a `Grouped`
    // when it is passed to `insert`
    #[doc(hidden)]
    fn requires_parenthesis(&self) -> bool {
        true
    }
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

impl<'a, T, Tab> Insertable<Tab> for &'a [T]
where
    &'a T: UndecoratedInsertRecord<Tab>,
{
    type Values = Self;

    fn values(self) -> Self::Values {
        self
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

impl<'a, T, Tab, DB> InsertValues<Tab, DB> for &'a [T]
where
    Tab: Table,
    DB: Backend + SupportsDefaultKeyword,
    &'a T: Insertable<Tab>,
    <&'a T as Insertable<Tab>>::Values: InsertValues<Tab, DB>,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> QueryResult<()> {
        self.get(0)
            .expect("Tried to read column names from empty list of rows")
            .values()
            .column_names(out)
    }

    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        for (i, record) in self.iter().enumerate() {
            if i != 0 {
                out.push_sql("), (");
            }
            record.values().walk_ast(out.reborrow())?;
        }
        Ok(())
    }

    fn is_noop(&self) -> bool {
        false
    }
}
