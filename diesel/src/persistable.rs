use std::marker::PhantomData;

use backend::{Backend, SupportsDefaultKeyword};
use expression::Expression;
use result::QueryResult;
use query_builder::{QueryBuilder, BuildQueryResult};
use query_source::{Table, Column};

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
    Expr: Expression<SqlType=Col::SqlType>,
{
    Expression(Col, Expr),
    Default(Col),
}

impl<'a, T, U: 'a, DB> Insertable<T, DB> for &'a [U] where
    T: Table,
    DB: Backend,
    &'a U: Insertable<T, DB>,
    DB: SupportsDefaultKeyword,
{
    type Values = BatchInsertValues<'a, T, U, DB>;

    fn values(self) -> Self::Values {
        BatchInsertValues {
            values: self,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, U, DB> Insertable<T, DB> for &'a Vec<U> where
    T: Table,
    DB: Backend,
    &'a [U]: Insertable<T, DB>,
{
    type Values = <&'a [U] as Insertable<T, DB>>::Values;

    fn values(self) -> Self::Values {
        (self as &'a [U]).values()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BatchInsertValues<'a, T, U: 'a, DB> {
    values: &'a [U],
    _marker: PhantomData<(T, DB)>,
}

impl<'a, T, U: 'a, DB> InsertValues<DB> for BatchInsertValues<'a, T, U, DB> where
    T: Table,
    DB: Backend,
    &'a U: Insertable<T, DB>,
{
    fn column_names(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.values[0].values().column_names(out)
    }

    fn values_clause(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        for (i, record) in self.values.into_iter().enumerate() {
            if i != 0 {
                out.push_sql(", ");
            }
            try!(record.values().values_clause(out));
        }
        Ok(())
    }

    fn values_bind_params(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        for record in self.values.into_iter() {
            try!(record.values().values_bind_params(out));
        }
        Ok(())
    }
}
