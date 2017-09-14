use std::marker::PhantomData;

use backend::{Backend, SupportsDefaultKeyword};
use result::QueryResult;
use query_builder::AstPass;
use query_source::Table;

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
    type Values: InsertValues<DB>;

    fn values(self) -> Self::Values;
}

pub trait InsertValues<DB: Backend> {
    fn column_names(&self, out: &mut DB::QueryBuilder) -> QueryResult<()>;
    fn walk_ast(&self, out: AstPass<DB>) -> QueryResult<()>;
}

#[derive(Debug, Copy, Clone)]
pub enum ColumnInsertValue<Col, Expr> {
    Expression(Col, Expr),
    Default(Col),
}

impl<'a, T, Tab, DB> Insertable<Tab, DB> for &'a [T]
where
    Tab: Table,
    DB: Backend + SupportsDefaultKeyword,
    &'a T: Insertable<Tab, DB>,
{
    type Values = BatchInsertValues<'a, T, Tab>;

    fn values(self) -> Self::Values {
        BatchInsertValues {
            records: self,
            _marker: PhantomData,
        }
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

#[derive(Debug, Clone)]
pub struct BatchInsertValues<'a, T: 'a, Tab> {
    records: &'a [T],
    _marker: PhantomData<Tab>,
}

impl<'a, T, Tab, DB> InsertValues<DB> for BatchInsertValues<'a, T, Tab>
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
}
