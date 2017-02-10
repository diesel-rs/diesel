use std::iter;

use backend::{Backend, SupportsDefaultKeyword};
use expression::Expression;
use result::QueryResult;
use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::{Table, Column};
use types::IntoNullable;

/// Represents that a structure can be used to to insert a new row into the
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
    fn column_names(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn values_clause(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn values_bind_params(&self, out: &mut DB::BindCollector) -> QueryResult<()>;
}

#[derive(Debug, Copy, Clone)]
pub enum ColumnInsertValue<Col, Expr> where
    Col: Column,
    Col::SqlType: IntoNullable,
    Expr: Expression<SqlType=<Col::SqlType as IntoNullable>::Nullable>,
{
    Expression(Col, Expr),
    Default(Col),
}

type ValuesFn<Item, T, DB> = fn(Item) -> <Item as Insertable<T, DB>>::Values;

impl<Iter, T, DB> Insertable<T, DB> for Iter where
    T: Table,
    DB: Backend + SupportsDefaultKeyword,
    Iter: IntoIterator,
    Iter::Item: Insertable<T, DB>,
    Iter::IntoIter: Clone,
{
    type Values = BatchInsertValues<iter::Map<
        Iter::IntoIter,
        ValuesFn<Iter::Item, T, DB>,
    >>;

    fn values(self) -> Self::Values {
        let values = self.into_iter()
            .map(Insertable::values as ValuesFn<Iter::Item, T, DB>);
        BatchInsertValues(values)
    }
}

#[derive(Debug, Clone)]
pub struct BatchInsertValues<T>(T);

impl<T, DB> InsertValues<DB> for BatchInsertValues<T> where
    T: Iterator + Clone,
    T::Item: InsertValues<DB>,
    DB: Backend,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.0.clone()
            .next().expect("Tried to read column names from empty list of rows")
            .column_names(out)
    }

    fn values_clause(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        for (i, values) in self.0.clone().enumerate() {
            if i != 0 {
                out.push_sql(", ");
            }
            try!(values.values_clause(out));
        }
        Ok(())
    }

    fn values_bind_params(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        for values in self.0.clone() {
            try!(values.values_bind_params(out));
        }
        Ok(())
    }
}
