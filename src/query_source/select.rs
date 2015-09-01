use QuerySource;
use types::NativeSqlType;
use std::marker::PhantomData;

pub struct SelectSqlQuerySource<A, S> {
    columns: String,
    source: S,
    _marker: PhantomData<A>,
}

impl<A, S> SelectSqlQuerySource<A, S> {
    pub fn new(columns: String, source: S) -> Self {
        SelectSqlQuerySource {
            columns: columns,
            source: source,
            _marker: PhantomData,
        }
    }
}


impl<A, S> QuerySource for SelectSqlQuerySource<A, S> where
    A: NativeSqlType,
    S: QuerySource,
{
    type SqlType = A;

    fn select_clause(&self) -> String {
        self.columns.clone()
    }

    fn from_clause(&self) -> String {
        self.source.from_clause()
    }
}
