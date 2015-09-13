use query_source::{Table, Column};
use types::ValuesToSql;

pub trait Insertable<T: Table, QS = T> {
    type Columns: Column<QS> + AsBindParam<QS>;
    type Values: ValuesToSql<<Self::Columns as Column<QS>>::SqlType>;

    fn columns() -> Self::Columns;

    fn values(self) -> Self::Values;
}

pub trait AsBindParam<T>: Column<T> {
    fn as_bind_param(idx: &mut usize) -> String;
}

impl<T: Table, C: Column<T>> AsBindParam<T> for C {
    fn as_bind_param(idx: &mut usize) -> String {
        let result = format!("${}", idx);
        *idx += 1;
        result
    }
}
