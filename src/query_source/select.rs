use QuerySource;
use expression::SelectableExpression;
use query_builder::*;
use std::marker::PhantomData;
use types::NativeSqlType;

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
    fn from_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        self.source.from_clause(out)
    }
}

impl<A, S, E> AsQuery for SelectSqlQuerySource<A, S, E> where
    A: NativeSqlType,
    S: QuerySource,
    E: SelectableExpression<S, A>,
{
    type SqlType = A;
    type Query = SelectStatement<A, E, S>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self.columns, self.source)
    }
}
