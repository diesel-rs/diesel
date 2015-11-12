use expression::Expression;
use query_source::{Table, Column};
use types::{ValuesToSql, NativeSqlType};

pub trait Insertable<'a, T: Table> {
    type Columns: InsertableColumns<T>;
    type Values: ValuesToSql<<Self::Columns as InsertableColumns<T>>::SqlType>
        + AsBindParam;

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

pub trait AsBindParam {
    fn as_bind_param(&self, idx: &mut usize) -> String {
        let result = format!("${}", idx);
        *idx += 1;
        result
    }

    fn as_bind_param_for_insert(&self, idx: &mut usize) -> String;
}

impl<T: AsBindParam> AsBindParam for Option<T> {
    fn as_bind_param_for_insert(&self, idx: &mut usize) -> String {
        match self {
            &Some(ref value) => value.as_bind_param_for_insert(idx),
            &None => "DEFAULT".to_string(),
        }
    }
}

impl<'a> AsBindParam for &'a str {
    fn as_bind_param_for_insert(&self, idx: &mut usize) -> String {
        self.as_bind_param(idx)
    }
}

impl<'a, T: AsBindParam> AsBindParam for &'a T {
    fn as_bind_param_for_insert(&self, idx: &mut usize) -> String {
        (*self).as_bind_param_for_insert(idx)
    }
}

macro_rules! as_bind_param {
    ($($Target:ty),+) => {$(
        impl AsBindParam for $Target {
            fn as_bind_param_for_insert(&self, idx: &mut usize) -> String {
                self.as_bind_param(idx)
            }
        }
    )+}
}

as_bind_param!(bool, i16, i32, i64, f32, f64, String);
