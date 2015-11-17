use expression::Expression;
use query_source::{Table, Column};
use types::NativeSqlType;

pub trait Insertable<'a, T: Table> {
    type Columns: InsertableColumns<T>;
    type Values: Expression<SqlType=<Self::Columns as InsertableColumns<T>>::SqlType>;

    fn columns() -> Self::Columns;

    fn values(&'a self) -> Self::Values;
}

pub trait InsertableColumns<T: Table> {
    type SqlType: NativeSqlType;

    fn names(&self) -> String;
}

impl<C: Column<Table=T>, T: Table> InsertableColumns<T> for C {
    type SqlType = <Self as Expression>::SqlType;

    fn names(&self) -> String {
        self.name()
    }
}
