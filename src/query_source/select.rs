use QuerySource;
use types::NativeSqlType;
use std::marker::PhantomData;
use expression::SelectableExpression;

#[derive(Copy, Clone)]
pub struct SelectSqlQuerySource<A, S, E> {
    columns: E,
    source: S,
    _marker: PhantomData<A>,
}

impl<A, S, E> SelectSqlQuerySource<A, S, E> {
    pub fn new(columns: E, source: S) -> Self {
        SelectSqlQuerySource {
            columns: columns,
            source: source,
            _marker: PhantomData,
        }
    }
}


impl<A, S, E> QuerySource for SelectSqlQuerySource<A, S, E> where
    A: NativeSqlType,
    S: QuerySource,
    E: SelectableExpression<S, A>,
{
    type SqlType = A;

    fn select_clause(&self) -> String {
        self.columns.to_sql()
    }

    fn from_clause(&self) -> String {
        self.source.from_clause()
    }
}
