use types::{FromSql, NativeSqlType};
use std::marker::PhantomData;

pub trait Queriable<QS: QuerySource> {
    type Row: FromSql<QS::SqlType>;

    fn build(row: Self::Row) -> Self;
}

pub unsafe trait QuerySource: Sized {
    type SqlType: NativeSqlType;

    fn select_clause(&self) -> &str;
    fn from_clause(&self) -> &str;

    unsafe fn select<A: NativeSqlType>(self, columns: &'static str) -> SelectedQuerySource<A, Self> {
        SelectedQuerySource {
            columns: columns,
            source: self,
            _marker: PhantomData,
        }
    }
}

pub struct SelectedQuerySource<A, S> where
    A: NativeSqlType,
    S: QuerySource,
{
    columns: &'static str,
    source: S,
    _marker: PhantomData<A>,
}

unsafe impl<A, S> QuerySource for SelectedQuerySource<A, S> where
    A: NativeSqlType,
    S: QuerySource,
{
    type SqlType = A;

    fn select_clause(&self) -> &str {
        self.columns
    }

    fn from_clause(&self) -> &str {
        self.source.from_clause()
    }
}
