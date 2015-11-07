use QuerySource;
use expression::SelectableExpression;
use query_builder::{QueryBuilder, BuildQueryResult};
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
    type SqlType = A;

    fn select_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        self.columns.to_sql(out)
    }

    fn from_clause(&self) -> String {
        self.source.from_clause()
    }

    fn where_clause<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        self.source.where_clause(out)
    }

    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select_clause(out));
        out.push_sql(" FROM ");
        out.push_sql(&self.from_clause());
        self.where_clause(out)
    }
}
